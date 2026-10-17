// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Standalone end-to-end runner, independent of the aifix runtime.
use aiagents::config::Config;
use serde::Serialize;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::{
    env, fs,
    io::{Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    process::{Child, Command, ExitCode, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Serialize)]
struct Attempt {
    run: usize,
    fix_seconds: f64,
    build_seconds: f64,
    fix_exit: Option<i32>,
    build_exit: Option<i32>,
    passed: bool,
    log: String,
}

#[derive(Serialize)]
struct Statistics {
    min: f64,
    max: f64,
    mean: f64,
    median: f64,
}

#[derive(Serialize)]
struct ResultRow {
    fixture: String,
    passed: usize,
    failed: usize,
    fix_seconds: Statistics,
    build_seconds: Option<Statistics>,
    attempts: Vec<Attempt>,
}

struct Service {
    child: Child,
    host: String,
    port: u16,
    stopped: bool,
}

impl Service {
    fn stop(&mut self) -> Result<(), String> {
        if self.stopped {
            return Ok(());
        }
        println!(
            "Stopping test model server on {}:{} (launcher PID {}) ...",
            self.host,
            self.port,
            self.child.id()
        );
        // aifix -r can launch a child model server. Killing only aifix
        // would leave that child listening on the shared model port.
        #[cfg(unix)]
        {
            let group = i32::try_from(self.child.id()).map_err(|err| err.to_string())?;
            // SAFETY: the spawned command is placed in its own process group.
            unsafe {
                libc::kill(-group, libc::SIGTERM);
            }
            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline {
                if TcpStream::connect((self.host.as_str(), self.port)).is_err() {
                    let _ = self.child.wait();
                    self.stopped = true;
                    println!("Model server stopped; port {} released.", self.port);
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(200));
            }
            eprintln!(
                "Model server did not stop after SIGTERM; sending SIGKILL to its process group"
            );
            // SAFETY: the process group was created exclusively for this run.
            unsafe {
                libc::kill(-group, libc::SIGKILL);
            }
        }
        #[cfg(windows)]
        {
            // taskkill /T terminates descendants as well as the launcher.
            let status = Command::new("taskkill")
                .args(["/PID", &self.child.id().to_string(), "/T", "/F"])
                .status()
                .map_err(|err| format!("Cannot execute taskkill: {err}"))?;
            if !status.success() {
                return Err(format!(
                    "taskkill failed for launcher PID {}: {status}",
                    self.child.id()
                ));
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if TcpStream::connect((self.host.as_str(), self.port)).is_err() {
                self.stopped = true;
                println!("Model server stopped; port {} released.", self.port);
                return Ok(());
            }
            thread::sleep(Duration::from_millis(200));
        }
        Err(format!(
            "Model server shutdown failed: port {} remains occupied",
            self.port
        ))
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        if let Err(err) = self.stop() {
            eprintln!("WARNING: {err}");
        }
    }
}

fn stats(values: &[f64]) -> Option<Statistics> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    Some(Statistics {
        min: sorted[0],
        max: sorted[n - 1],
        mean: sorted.iter().sum::<f64>() / n as f64,
        median: if n.is_multiple_of(2) {
            (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
        } else {
            sorted[n / 2]
        },
    })
}

fn execute(dir: &Path, args: &[&str], extra_args: Option<&str>) -> (Option<i32>, f64, String) {
    let start = Instant::now();
    let mut cmd = Command::new(args[0]);
    cmd.args(&args[1..]).current_dir(dir);
    if let Some(extra) = extra_args {
        cmd.env("AIFIX_TEST_ARGS", extra);
    }
    match cmd.output() {
        Ok(output) => {
            let mut log = String::from_utf8_lossy(&output.stdout).into_owned();
            log.push_str(&String::from_utf8_lossy(&output.stderr));
            (output.status.code(), start.elapsed().as_secs_f64(), log)
        }
        Err(err) => (
            None,
            start.elapsed().as_secs_f64(),
            format!("Cannot run {}: {err}\n", args[0]),
        ),
    }
}

fn health_url(endpoint: &str) -> Result<(String, u16), String> {
    let addr = endpoint
        .strip_prefix("http://")
        .ok_or("Only http:// local model endpoints are supported")?;
    let authority = addr.split('/').next().ok_or("Missing endpoint authority")?;
    let (host, port) = authority
        .rsplit_once(':')
        .ok_or("Endpoint needs an explicit port")?;
    let port = port.parse::<u16>().map_err(|_| "Invalid endpoint port")?;
    if host != "localhost" && host != "127.0.0.1" && host != "[::1]" {
        return Err("Runner may start only a local model endpoint".into());
    }
    Ok((
        if host == "[::1]" {
            "[::1]".into()
        } else {
            host.into()
        },
        port,
    ))
}

enum Health {
    Ready,
    Starting,
    Fatal(u16),
    Unreachable,
}

fn health(host: &str, port: u16) -> Health {
    let Ok(mut stream) = TcpStream::connect((host, port)) else {
        return Health::Unreachable;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    if stream
        .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return Health::Unreachable;
    }
    let mut buf = [0u8; 256];
    let Ok(n) = stream.read(&mut buf) else {
        return Health::Unreachable;
    };
    let response = String::from_utf8_lossy(&buf[..n]);
    let Some(status) = response
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
    else {
        return Health::Unreachable;
    };
    match status {
        200 => Health::Ready,
        503 => Health::Starting,
        500..=599 => Health::Fatal(status),
        _ => Health::Fatal(status),
    }
}

#[cfg(target_os = "linux")]
fn listener_pids(port: u16) -> Vec<u32> {
    use std::collections::BTreeSet;
    let mut inodes = BTreeSet::new();
    for table in ["/proc/net/tcp", "/proc/net/tcp6"] {
        if let Ok(contents) = fs::read_to_string(table) {
            for line in contents.lines().skip(1) {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() < 10 || fields[3] != "0A" {
                    continue;
                }
                let Some((_, hex_port)) = fields[1].rsplit_once(':') else {
                    continue;
                };
                if u16::from_str_radix(hex_port, 16).ok() == Some(port) {
                    inodes.insert(fields[9].to_owned());
                }
            }
        }
    }
    let mut pids = BTreeSet::new();
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
                continue;
            };
            let Ok(fds) = fs::read_dir(entry.path().join("fd")) else {
                continue;
            };
            for fd in fds.flatten() {
                let Ok(link) = fs::read_link(fd.path()) else {
                    continue;
                };
                let target = link.to_string_lossy();
                if let Some(inode) = target
                    .strip_prefix("socket:[")
                    .and_then(|s| s.strip_suffix(']'))
                    && inodes.contains(inode)
                {
                    pids.insert(pid);
                    break;
                }
            }
        }
    }
    pids.into_iter().collect()
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("aifix_runtest: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut report_dir = None;
    let mut model: Option<String> = None;
    let mut all = false;
    let mut config_path: Option<PathBuf> = None;
    let mut runs = 1usize;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = PathBuf::from(args.next().ok_or("--root needs a directory")?),
            "--report-dir" => {
                report_dir = Some(PathBuf::from(
                    args.next().ok_or("--report-dir needs a directory")?,
                ))
            }
            "--model" => model = Some(args.next().ok_or("--model needs a provider name")?),
            "-a" | "--all" => all = true,
            "--config" => {
                config_path = Some(PathBuf::from(args.next().ok_or("--config needs a path")?))
            }
            "--runs" => {
                runs = args.next().ok_or("--runs needs a number")?.parse()?;
                if runs == 0 {
                    return Err("--runs must be greater than zero".into());
                }
            }
            "-h" | "--help" => {
                println!(
                    "aifix_runtest [--all | -a | --model PROVIDER] [--runs N] [--config PATH] [--root REPO] [--report-dir DIR]\nWARNING: fixture-local tracked changes are reset before each attempt."
                );
                return Ok(true);
            }
            _ => return Err(format!("unknown option: {arg}").into()),
        }
    }
    root = root.canonicalize()?;
    if !root.join("runtests").is_dir() {
        return Err("root must contain runtests/".into());
    }
    let reports = report_dir.unwrap_or_else(|| root.join("target/runtest-reports"));
    fs::create_dir_all(&reports)?;

    let config = Config::load_or_create(
        config_path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned()),
    )
    .map_err(|e| format!("Cannot load config: {e:?}"))?;
    if all {
        if model.is_some() {
            return Err("--all and --model cannot be used together".into());
        }
        let selected: Vec<&str> = config
            .providerlist
            .iter()
            .filter(|provider| provider.active)
            .map(|provider| provider.name.as_str())
            .collect();
        if selected.is_empty() {
            return Err("--all: no providers with active=true in the configuration".into());
        }
        println!("Active models to test ({}):", selected.len());
        for (index, provider) in config
            .providerlist
            .iter()
            .filter(|provider| provider.active)
            .enumerate()
        {
            println!("  {}. {} — {}", index + 1, provider.name, provider.model);
        }
        let executable = env::current_exe()?;
        let mut all_passed = true;
        for (index, name) in selected.iter().enumerate() {
            if !name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
            {
                return Err(format!("Invalid provider name for report directory: {name}").into());
            }
            let model_reports = reports.join(name);
            fs::create_dir_all(&model_reports)?;
            println!(
                "\n=== Provider {}/{}: {} ===",
                index + 1,
                selected.len(),
                name
            );
            let mut cmd = Command::new(&executable);
            cmd.arg("--model")
                .arg(name)
                .arg("--runs")
                .arg(runs.to_string())
                .arg("--root")
                .arg(&root)
                .arg("--report-dir")
                .arg(&model_reports);
            if let Some(ref path) = config_path {
                cmd.arg("--config").arg(path);
            }
            let status = cmd
                .status()
                .map_err(|err| format!("Cannot start runner for {name}: {err}"))?;
            if !status.success() {
                eprintln!(
                    "ERROR: Provider '{name}' failed (exit status: {status}). Reports: {}",
                    model_reports.display()
                );
                all_passed = false;
            }
        }
        println!(
            "All-provider run complete. Individual reports: {}",
            reports.display()
        );
        return Ok(all_passed);
    }

    let provider = match model.as_deref() {
        Some(name) => config
            .get_provider(name)
            .ok_or_else(|| format!("Unknown provider: {name}"))?,
        None => config
            .get_selected_provider()
            .ok_or("No selected provider")?,
    };
    let selected_name = provider.name.clone();
    let mut service: Option<Service> = None;
    if model.is_some() {
        let (host, port) = health_url(&provider.endpoint)?;
        if TcpStream::connect((host.as_str(), port)).is_ok() {
            #[cfg(target_os = "linux")]
            let pid_info = {
                let pids = listener_pids(port);
                if pids.is_empty() {
                    "PID unavailable (possibly insufficient /proc permissions)".to_owned()
                } else {
                    format!(
                        "Listening PID(s): {}",
                        pids.iter()
                            .map(u32::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            };
            #[cfg(not(target_os = "linux"))]
            let pid_info = "PID unavailable on this platform".to_owned();
            #[cfg(windows)]
            let pid_info = {
                let output = Command::new("netstat").args(["-ano", "-p", "TCP"]).output();
                match output {
                    Ok(output) => {
                        let pids: std::collections::BTreeSet<u32> =
                            String::from_utf8_lossy(&output.stdout)
                                .lines()
                                .filter_map(|line| {
                                    let fields: Vec<&str> = line.split_whitespace().collect();
                                    if fields.len() < 5 || fields[3] != "LISTENING" {
                                        return None;
                                    }
                                    let (_, local_port) = fields[1].rsplit_once(':')?;
                                    (local_port.parse::<u16>().ok() == Some(port))
                                        .then(|| fields[4].parse::<u32>().ok())
                                        .flatten()
                                })
                                .collect();
                        if pids.is_empty() {
                            "PID unavailable".to_owned()
                        } else {
                            format!(
                                "Listening PID(s): {}",
                                pids.iter()
                                    .map(u32::to_string)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        }
                    }
                    Err(_) => "PID unavailable (netstat failed)".to_owned(),
                }
            };
            #[cfg(windows)]
            let instructions = format!(
                "Verify the process: netstat -ano | findstr :{port}\nInspect the PID, then stop only that service: taskkill /PID <PID> /T /F"
            );
            #[cfg(not(windows))]
            let instructions = format!(
                "Verify the process: ss -ltnp '( sport = :{port} )'\nInspect the PID, then stop only that service: kill <PID>"
            );
            return Err(format!("Endpoint {host}:{port} is already occupied; refusing to assume which model is running.\n{pid_info}\n{instructions}").into());
        }
        let aifix = env::current_exe()?.with_file_name("aifix");
        // A previous model may have released its port before the GPU driver
        // has reclaimed all memory. Retry only a startup that actually failed.
        let mut ready = false;
        for attempt in 1..=3 {
            if attempt > 1 {
                println!(
                    "Waiting 15s for GPU memory reclamation before startup retry {attempt}/3 ..."
                );
                thread::sleep(Duration::from_secs(15));
            }
            let mut cmd = Command::new(&aifix);
            if let Some(ref path) = config_path {
                cmd.arg("-c").arg(path.canonicalize()?);
            }
            let server_log = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(reports.join("model-server.log"))?;
            cmd.arg("-r")
                .arg(&selected_name)
                .stdout(Stdio::null())
                .stderr(Stdio::from(server_log));
            #[cfg(unix)]
            cmd.process_group(0);
            let child = cmd.spawn().map_err(|err| {
                format!("Cannot launch model service via {}: {err}", aifix.display())
            })?;
            service = Some(Service {
                child,
                host: host.clone(),
                port,
                stopped: false,
            });
            let deadline = Instant::now() + Duration::from_secs(180);
            let mut failure = String::new();
            loop {
                match health(&host, port) {
                    Health::Ready => {
                        ready = true;
                        break;
                    }
                    Health::Fatal(status) => {
                        failure = format!("Model server health check returned HTTP {status}");
                        break;
                    }
                    Health::Starting | Health::Unreachable => {}
                }
                if service.as_mut().unwrap().child.try_wait()?.is_some() {
                    failure = "Model server exited before becoming ready".into();
                    break;
                }
                if Instant::now() >= deadline {
                    failure = "Model server did not become ready within 180 seconds".into();
                    break;
                }
                thread::sleep(Duration::from_millis(500));
            }
            if ready {
                break;
            }
            eprintln!("Startup attempt {attempt}/3 failed: {failure}; see model-server.log");
            // The child may already have exited. Never kill unrelated port owners.
            if let Some(mut running) = service.take() {
                let _ = running.stop();
            }
            if attempt == 3 {
                return Err(format!(
                    "{failure}; exhausted 3 startup attempts; see model-server.log"
                )
                .into());
            }
        }
    }

    let config_arg = config_path.as_ref().map(|p| p.canonicalize()).transpose()?;
    if !selected_name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("Invalid provider name".into());
    }
    let mut extra = format!("-p {}", selected_name);
    if let Some(ref path) = config_arg {
        extra.push_str(&format!(" -c {}", path.display()));
        if path.to_string_lossy().contains(char::is_whitespace) {
            return Err("Config path with whitespace is not supported".into());
        }
    }
    if extra.chars().any(|c| c == '\n' || c == '\r') {
        return Err("Invalid provider or config path".into());
    }

    let mut fixtures = Vec::new();
    for system in ["cargo", "cmake", "meson", "maven"] {
        let base = root.join("runtests").join(system);
        if !base.exists() {
            continue;
        }
        for entry in fs::read_dir(base)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() && entry.path().join("Justfile").is_file() {
                fixtures.push((
                    format!("{system}/{}", entry.file_name().to_string_lossy()),
                    entry.path(),
                ));
            }
        }
    }
    fixtures.sort_by(|a, b| a.0.cmp(&b.0));
    if fixtures.is_empty() {
        return Err("no runtest fixtures found".into());
    }

    let mut rows = Vec::new();
    for (name, dir) in fixtures {
        let mut attempts = Vec::new();
        println!(
            "Running {name} | provider: {} | model: {} ({runs} repetitions) ...",
            selected_name, provider.model
        );
        for number in 1..=runs {
            let (fix_exit, fix_seconds, fix_log) = execute(&dir, &["just", "fix"], Some(&extra));
            // A dead server cannot repair any remaining fixture. Preserve its
            // failure in the current attempt and stop this provider early.
            let server_gone = service.as_mut().is_some_and(|running| {
                running.child.try_wait().ok().flatten().is_some()
                    || TcpStream::connect((running.host.as_str(), running.port)).is_err()
            });
            if server_gone {
                eprintln!(
                    "Model server became unavailable during {name} run {number}; aborting provider. See model-server.log"
                );
            }
            let (build_exit, build_seconds, build_log) = if fix_exit == Some(0) {
                execute(&dir, &["just", "build"], None)
            } else {
                (None, 0.0, "Skipped: fix failed\n".into())
            };
            let passed = fix_exit == Some(0) && build_exit == Some(0);
            let log = format!("{}_run{}.log", name.replace('/', "_"), number);
            fs::write(
                reports.join(&log),
                format!("=== FIX ===\n{fix_log}\n=== BUILD ===\n{build_log}"),
            )?;
            println!(
                "  run {number}: {} ({fix_seconds:.2}s fix, {build_seconds:.2}s build)",
                if passed { "PASS" } else { "FAIL" }
            );
            attempts.push(Attempt {
                run: number,
                fix_seconds,
                build_seconds,
                fix_exit,
                build_exit,
                passed,
                log,
            });
            if server_gone {
                return Err(format!("Provider {selected_name}: model server exited or stopped listening during {name} run {number}; see {}", reports.join("model-server.log").display()).into());
            }
            // Fixtures contain tracked, deliberately broken sources. Ignore rules
            // cannot hide their modifications, so restore them after each attempt.
            // The fixture's 'just fix' already restores these files before testing.
            let reset = Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(["restore", "--worktree", "--", "."])
                .output()?;
            if !reset.status.success() {
                return Err(format!(
                    "Cannot restore fixture {name} after run {number}: {}",
                    String::from_utf8_lossy(&reset.stderr)
                )
                .into());
            }
        }
        let passed = attempts.iter().filter(|a| a.passed).count();
        let fix = stats(&attempts.iter().map(|a| a.fix_seconds).collect::<Vec<_>>()).unwrap();
        let build = stats(
            &attempts
                .iter()
                .filter(|a| a.build_exit.is_some())
                .map(|a| a.build_seconds)
                .collect::<Vec<_>>(),
        );
        rows.push(ResultRow {
            fixture: name,
            passed,
            failed: runs - passed,
            fix_seconds: fix,
            build_seconds: build,
            attempts,
        });
    }
    let total = rows.len() * runs;
    let passed: usize = rows.iter().map(|r| r.passed).sum();
    let elapsed: f64 = rows
        .iter()
        .flat_map(|r| &r.attempts)
        .map(|a| a.fix_seconds + a.build_seconds)
        .sum();
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let model_metadata = serde_json::json!({
        "provider": selected_name,
        "model": provider.model,
        "source": provider.source,
        "endpoint": provider.endpoint,
        "llmbin": provider.llmbin,
        "llmparam": provider.llmparam,
        "note": "Model filename may encode quantization; no independent model identity verification"
    });
    let json = serde_json::json!({"timestamp_unix":stamp,"model":model_metadata,"runs":runs,"total":total,"passed":passed,"failed":total-passed,"elapsed_seconds":elapsed,"results":rows});
    fs::write(
        reports.join("results.json"),
        serde_json::to_string_pretty(&json)?,
    )?;
    let mut md = format!(
        "# Runtime test results\n\nProvider: {} | repetitions: {runs} | passed: {passed}/{total} | command time: {elapsed:.2}s\n\n| Fixture | Pass | Fix min | Fix max | Fix mean | Fix median |\n| --- | ---: | ---: | ---: | ---: | ---: |\n",
        selected_name
    );
    for row in &rows {
        md.push_str(&format!(
            "| {} | {}/{} | {:.2} | {:.2} | {:.2} | {:.2} |\n",
            row.fixture,
            row.passed,
            runs,
            row.fix_seconds.min,
            row.fix_seconds.max,
            row.fix_seconds.mean,
            row.fix_seconds.median
        ));
    }
    fs::write(reports.join("results.md"), md)?;
    // Publish compact, versionable reports outside the ignored target tree.
    // Keep verbose fixture and server logs in target/runtest-reports.
    let snapshot_dir = root.join("runtests").join("results").join(&selected_name);
    fs::create_dir_all(&snapshot_dir)?;
    fs::copy(
        reports.join("results.json"),
        snapshot_dir.join("results.json"),
    )?;
    fs::copy(reports.join("results.md"), snapshot_dir.join("results.md"))?;
    println!("Report snapshots: {}", snapshot_dir.display());
    println!(
        "Results: {passed}/{total} passed; reports in {}",
        reports.display()
    );
    if let Some(mut running) = service.take() {
        running
            .stop()
            .map_err(|err| format!("Provider {selected_name}: {err}"))?;
    }
    Ok(passed == total)
}
