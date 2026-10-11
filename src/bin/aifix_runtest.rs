// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Standalone end-to-end runner, independent of the aifix runtime.
use std::{env, fs, io::{Read, Write}, net::TcpStream, path::{Path, PathBuf}, process::{Child, Command, ExitCode, Stdio}, thread, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use serde::Serialize;
use aiagents::config::Config;

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
struct Statistics { min: f64, max: f64, mean: f64, median: f64 }

#[derive(Serialize)]
struct ResultRow {
    fixture: String,
    passed: usize,
    failed: usize,
    fix_seconds: Statistics,
    build_seconds: Option<Statistics>,
    attempts: Vec<Attempt>,
}

struct Service(Child);
impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn stats(values: &[f64]) -> Option<Statistics> {
    if values.is_empty() { return None; }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    Some(Statistics {
        min: sorted[0],
        max: sorted[n - 1],
        mean: sorted.iter().sum::<f64>() / n as f64,
        median: if n.is_multiple_of(2) { (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0 } else { sorted[n / 2] },
    })
}

fn execute(dir: &Path, args: &[&str], extra_args: Option<&str>) -> (Option<i32>, f64, String) {
    let start = Instant::now();
    let mut cmd = Command::new(args[0]);
    cmd.args(&args[1..]).current_dir(dir);
    if let Some(extra) = extra_args { cmd.env("AIFIX_TEST_ARGS", extra); }
    match cmd.output() {
        Ok(output) => {
            let mut log = String::from_utf8_lossy(&output.stdout).into_owned();
            log.push_str(&String::from_utf8_lossy(&output.stderr));
            (output.status.code(), start.elapsed().as_secs_f64(), log)
        }
        Err(err) => (None, start.elapsed().as_secs_f64(), format!("Cannot run {}: {err}\n", args[0])),
    }
}

fn health_url(endpoint: &str) -> Result<(String, u16), String> {
    let addr = endpoint.strip_prefix("http://").ok_or("Only http:// local model endpoints are supported")?;
    let authority = addr.split('/').next().ok_or("Missing endpoint authority")?;
    let (host, port) = authority.rsplit_once(':').ok_or("Endpoint needs an explicit port")?;
    let port = port.parse::<u16>().map_err(|_| "Invalid endpoint port")?;
    if host != "localhost" && host != "127.0.0.1" && host != "[::1]" {
        return Err("Runner may start only a local model endpoint".into());
    }
    Ok((if host == "[::1]" { "[::1]".into() } else { host.into() }, port))
}

enum Health {
    Ready,
    Starting,
    Fatal(u16),
    Unreachable,
}

fn health(host: &str, port: u16) -> Health {
    let Ok(mut stream) = TcpStream::connect((host, port)) else { return Health::Unreachable };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    if stream.write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").is_err() {
        return Health::Unreachable;
    }
    let mut buf = [0u8; 256];
    let Ok(n) = stream.read(&mut buf) else { return Health::Unreachable };
    let response = String::from_utf8_lossy(&buf[..n]);
    let Some(status) = response.lines().next().and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok()) else {
        return Health::Unreachable;
    };
    match status {
        200 => Health::Ready,
        503 => Health::Starting,
        500..=599 => Health::Fatal(status),
        _ => Health::Fatal(status),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(err) => { eprintln!("aifix_runtest: {err}"); ExitCode::FAILURE }
    }
}

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut report_dir = None;
    let mut model: Option<String> = None;
    let mut config_path: Option<PathBuf> = None;
    let mut runs = 1usize;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = PathBuf::from(args.next().ok_or("--root needs a directory")?),
            "--report-dir" => report_dir = Some(PathBuf::from(args.next().ok_or("--report-dir needs a directory")?)),
            "--model" => model = Some(args.next().ok_or("--model needs a provider name")?),
            "--config" => config_path = Some(PathBuf::from(args.next().ok_or("--config needs a path")?)),
            "--runs" => {
                runs = args.next().ok_or("--runs needs a number")?.parse()?;
                if runs == 0 { return Err("--runs must be greater than zero".into()); }
            }
            "-h" | "--help" => {
                println!("aifix_runtest [--model PROVIDER] [--runs N] [--config PATH] [--root REPO] [--report-dir DIR]\nWARNING: fixture-local tracked changes are reset before each attempt.");
                return Ok(true);
            }
            _ => return Err(format!("unknown option: {arg}").into()),
        }
    }
    root = root.canonicalize()?;
    if !root.join("runtests").is_dir() { return Err("root must contain runtests/".into()); }
    let reports = report_dir.unwrap_or_else(|| root.join("target/runtest-reports"));
    fs::create_dir_all(&reports)?;

    let config = Config::load_or_create(config_path.as_ref().map(|p| p.to_string_lossy().into_owned())).map_err(|e| format!("Cannot load config: {e:?}"))?;
    let provider = match model.as_deref() {
        Some(name) => config.get_provider(name).ok_or_else(|| format!("Unknown provider: {name}"))?,
        None => config.get_selected_provider().ok_or("No selected provider")?,
    };
    let selected_name = provider.name.clone();
    let mut service: Option<Service> = None;
    if model.is_some() {
        let (host, port) = health_url(&provider.endpoint)?;
        if TcpStream::connect((host.as_str(), port)).is_ok() {
            return Err(format!("Endpoint {host}:{port} is already occupied; refusing to assume which model is running").into());
        }
        let aifix = env::current_exe()?.with_file_name("aifix");
        let mut cmd = Command::new(&aifix);
        if let Some(ref path) = config_path { cmd.arg("-c").arg(path.canonicalize()?); }
        let server_log = fs::File::create(reports.join("model-server.log"))?;
        cmd.arg("-r").arg(&selected_name).stdout(Stdio::null()).stderr(Stdio::from(server_log));
        let child = cmd.spawn().map_err(|err| format!("Cannot launch model service via {}: {err}", aifix.display()))?;
        service = Some(Service(child));
        let deadline = Instant::now() + Duration::from_secs(180);
        loop {
            match health(&host, port) {
                Health::Ready => break,
                Health::Fatal(status) => return Err(format!("Model server health check returned HTTP {status}; see model-server.log").into()),
                Health::Starting | Health::Unreachable => {},
            }
            if service.as_mut().unwrap().0.try_wait()?.is_some() {
                return Err("Model server exited before becoming ready; see model-server.log".into());
            }
            if Instant::now() >= deadline { return Err("Model server did not become ready within 180 seconds".into()); }
            thread::sleep(Duration::from_millis(500));
        }
    }

    let config_arg = config_path.as_ref().map(|p| p.canonicalize()).transpose()?;
    if !selected_name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') { return Err("Invalid provider name".into()); }
    let mut extra = format!("-p {}", selected_name);
    if let Some(ref path) = config_arg {
        extra.push_str(&format!(" -c {}", path.display()));
        if path.to_string_lossy().contains(char::is_whitespace) { return Err("Config path with whitespace is not supported".into()); }
    }
    if extra.chars().any(|c| c == '\n' || c == '\r') { return Err("Invalid provider or config path".into()); }

    let mut fixtures = Vec::new();
    for system in ["cargo", "cmake", "meson", "maven"] {
        let base = root.join("runtests").join(system);
        if !base.exists() { continue; }
        for entry in fs::read_dir(base)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() && entry.path().join("Justfile").is_file() {
                fixtures.push((format!("{system}/{}", entry.file_name().to_string_lossy()), entry.path()));
            }
        }
    }
    fixtures.sort_by(|a, b| a.0.cmp(&b.0));
    if fixtures.is_empty() { return Err("no runtest fixtures found".into()); }

    let mut rows = Vec::new();
    for (name, dir) in fixtures {
        let mut attempts = Vec::new();
        println!("Running {name} ({runs} repetitions) ...");
        for number in 1..=runs {
            let (fix_exit, fix_seconds, fix_log) = execute(&dir, &["just", "fix"], Some(&extra));
            let (build_exit, build_seconds, build_log) = if fix_exit == Some(0) {
                execute(&dir, &["just", "build"], None)
            } else {
                (None, 0.0, "Skipped: fix failed\n".into())
            };
            let passed = fix_exit == Some(0) && build_exit == Some(0);
            let log = format!("{}_run{}.log", name.replace('/', "_"), number);
            fs::write(reports.join(&log), format!("=== FIX ===\n{fix_log}\n=== BUILD ===\n{build_log}"))?;
            println!("  run {number}: {} ({fix_seconds:.2}s fix, {build_seconds:.2}s build)", if passed {"PASS"} else {"FAIL"});
            attempts.push(Attempt {run: number, fix_seconds, build_seconds, fix_exit, build_exit, passed, log});
        }
        let passed = attempts.iter().filter(|a| a.passed).count();
        let fix = stats(&attempts.iter().map(|a| a.fix_seconds).collect::<Vec<_>>()).unwrap();
        let build = stats(&attempts.iter().filter(|a| a.build_exit.is_some()).map(|a| a.build_seconds).collect::<Vec<_>>());
        rows.push(ResultRow {fixture:name, passed, failed:runs-passed, fix_seconds:fix, build_seconds:build, attempts});
    }
    let total = rows.len() * runs;
    let passed: usize = rows.iter().map(|r| r.passed).sum();
    let elapsed: f64 = rows.iter().flat_map(|r| &r.attempts).map(|a| a.fix_seconds + a.build_seconds).sum();
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
    fs::write(reports.join("results.json"), serde_json::to_string_pretty(&json)?)?;
    let mut md = format!("# Runtime test results\n\nProvider: {} | repetitions: {runs} | passed: {passed}/{total} | command time: {elapsed:.2}s\n\n| Fixture | Pass | Fix min | Fix max | Fix mean | Fix median |\n| --- | ---: | ---: | ---: | ---: | ---: |\n", selected_name);
    for row in &rows {
        md.push_str(&format!("| {} | {}/{} | {:.2} | {:.2} | {:.2} | {:.2} |\n", row.fixture, row.passed, runs, row.fix_seconds.min, row.fix_seconds.max, row.fix_seconds.mean, row.fix_seconds.median));
    }
    fs::write(reports.join("results.md"), md)?;
    println!("Results: {passed}/{total} passed; reports in {}", reports.display());
    drop(service);
    Ok(passed == total)
}
