// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Standalone end-to-end runner. Deliberately independent of the aifix runtime.
use std::{env, fs, path::{Path, PathBuf}, process::{Command, ExitCode}, time::{Instant, SystemTime, UNIX_EPOCH}};
use serde::Serialize;

#[derive(Serialize)]
struct ResultRow {
    fixture: String,
    fix_seconds: f64,
    build_seconds: f64,
    fix_exit: Option<i32>,
    build_exit: Option<i32>,
    passed: bool,
    log: String,
}

fn run(dir: &Path, args: &[&str]) -> std::io::Result<(Option<i32>, f64, String)> {
    let start = Instant::now();
    let output = Command::new(args[0]).args(&args[1..]).current_dir(dir).output()?;
    let mut log = String::from_utf8_lossy(&output.stdout).into_owned();
    log.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok((output.status.code(), start.elapsed().as_secs_f64(), log))
}

fn execute(dir: &Path, args: &[&str]) -> (Option<i32>, f64, String) {
    run(dir, args).unwrap_or_else(|err| (None, 0.0, format!("Cannot run {}: {err}\n", args[0])))
}

fn main() -> ExitCode {
    match main_result() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(err) => { eprintln!("aifix_runtest: {err}"); ExitCode::FAILURE }
    }
}

fn main_result() -> Result<bool, Box<dyn std::error::Error>> {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut report_dir: Option<PathBuf> = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = PathBuf::from(args.next().ok_or("--root needs a directory")?),
            "--report-dir" => report_dir = Some(PathBuf::from(args.next().ok_or("--report-dir needs a directory")?)),
            "-h" | "--help" => {
                println!("aifix_runtest [--root REPO] [--report-dir DIR]\nRuns all runtests fixtures sequentially using just fix and just build.\nWARNING: just fix resets tracked changes within the fixture only.");
                return Ok(true);
            }
            _ => return Err(format!("unknown option: {arg}").into()),
        }
    }
    root = root.canonicalize()?;
    if !root.join("runtests").is_dir() {
        return Err("root must contain runtests/".into());
    }
    let reports = report_dir.unwrap_or_else(|| root.join("target").join("runtest-reports"));
    fs::create_dir_all(&reports)?;
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
        println!("Running {name} ...");
        // Each Justfile restores only its own fixture before invoking aifix.
        let (fix_exit, fix_seconds, fix_log) = execute(&dir, &["just", "fix"]);
        let (build_exit, build_seconds, build_log) = if fix_exit == Some(0) {
            execute(&dir, &["just", "build"])
        } else {
            (None, 0.0, "Skipped: fix failed\n".into())
        };
        let passed = fix_exit == Some(0) && build_exit == Some(0);
        let slug = name.replace('/', "_");
        let log_name = format!("{slug}.log");
        fs::write(reports.join(&log_name), format!("=== FIX ===\n{fix_log}\n=== BUILD ===\n{build_log}"))?;
        println!("  {} ({:.2}s fix, {:.2}s build)", if passed {"PASS"} else {"FAIL"}, fix_seconds, build_seconds);
        rows.push(ResultRow {fixture: name, fix_seconds, build_seconds, fix_exit, build_exit, passed, log: log_name});
    }
    let total = rows.len();
    let passed = rows.iter().filter(|r| r.passed).count();
    let elapsed: f64 = rows.iter().map(|r| r.fix_seconds + r.build_seconds).sum();
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let json = serde_json::json!({"timestamp_unix": stamp, "total":total, "passed":passed, "failed":total-passed, "elapsed_seconds":elapsed, "results":rows});
    fs::write(reports.join("results.json"), serde_json::to_string_pretty(&json)?)?;
    let mut markdown = format!("# Runtime test results\n\nPassed: {passed}/{total}; summed command time: {elapsed:.2}s\n\n| Fixture | Result | Fix (s) | Build (s) | Log |\n| --- | --- | ---: | ---: | --- |\n");
    for r in &rows {
        markdown.push_str(&format!("| {} | {} | {:.2} | {:.2} | {} |\n", r.fixture, if r.passed {"PASS"} else {"FAIL"}, r.fix_seconds, r.build_seconds, r.log));
    }
    fs::write(reports.join("results.md"), markdown)?;
    println!("Results: {passed}/{total} passed; reports in {}", reports.display());
    Ok(passed == total)
}
