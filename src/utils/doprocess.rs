// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::path::Path;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::env;

pub struct DoProcess {
    cmd: Vec<String>,
    cwd: Option<std::path::PathBuf>,
}

impl DoProcess {
    pub fn from_str_slice(cmd: &[&str], cwd: &Path) -> Self {
        Self {
            cmd: cmd.iter().map(|s| s.to_string()).collect(),
            cwd: Some(cwd.to_path_buf()),
        }
    }

    pub fn from_string_vec(cmd: &[String], cwd: &Path) -> Self {
        Self {
            cmd: cmd.to_vec(),
            cwd: Some(cwd.to_path_buf()),
        }
    }

    /// Runs the process and collects stdout/stderr into line buffers.
    /// Returns (exit_code, stdout_lines, stderr_lines)
    pub fn run_to_lines_separated(
        &self,
        out_lines: &mut Vec<String>,
        err_lines: &mut Vec<String>,
    ) -> (i32, Vec<String>, Vec<String>) {
        if self.cmd.is_empty() {
            return (0i32, Vec::<String>::new(), Vec::<String>::new());
        }
        let mut command = Command::new(&self.cmd[0]);

        if self.cmd.len() > 1 {
            command.args(&self.cmd[1..]);
        }

        if let Some(ref dir) = self.cwd {
            command.current_dir(dir);
        }

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn().expect("failed to spawn process");

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let mut out_reader = BufReader::new(stdout);
        let mut err_reader = BufReader::new(stderr);

        let mut out_buf = String::new();
        let mut err_buf = String::new();

        while out_reader.read_line(&mut out_buf).unwrap() > 0 {
            out_lines.push(out_buf.trim_end().to_string());
            out_buf.clear();
        }

        while err_reader.read_line(&mut err_buf).unwrap() > 0 {
            err_lines.push(err_buf.trim_end().to_string());
            err_buf.clear();
        }

        let status = child.wait().expect("failed to wait for process");

        let code = status.code().unwrap_or(-1);

        (code, out_lines.clone(), err_lines.clone())
    }


    /// Runs the process and collects stdout/stderr into line buffers.
    /// Returns (exit_code, stdout_lines, stderr_lines)
    pub fn run_to_lines_combined(&self, lines: &mut Vec<String>) -> (i32, Vec<String>) {
        if self.cmd.is_empty() {
            return (0i32, Vec::<String>::new());
        }

        let mut command = Command::new(&self.cmd[0]);

        if self.cmd.len() > 1 {
            command.args(&self.cmd[1..]);
        }

        if let Some(ref dir) = self.cwd {
            command.current_dir(dir);
        }

        // Merge RUSTFLAGS instead of overwriting them
        if let Ok(current_dir) = env::current_dir()
            && let Some(dir_str) = current_dir.to_str()
        {
            let remap = format!("--remap-path-prefix src={}/src", dir_str);

            // Preserve existing RUSTFLAGS (important!)
            let merged = match env::var("RUSTFLAGS") {
                Ok(existing) => format!("{} {} -Dwarnings", existing, remap),
                Err(_) => format!("{} -Dwarnings", remap),
            };
            command.env("RUSTFLAGS", merged);
        }

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn().expect("failed to spawn process");

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // Read stdout in a thread
        let out_handle = std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            reader
                .lines()
                .map(|l| l.unwrap())
                .collect::<Vec<String>>()
        });

        // Read stderr in a thread
        let err_handle = std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            reader
                .lines()
                .map(|l| l.unwrap())
                .collect::<Vec<String>>()
        });

        let out_lines = out_handle.join().unwrap();
        let err_lines = err_handle.join().unwrap();

        // Merge
        for l in out_lines {
            lines.push(l);
        }
        for l in err_lines {
            lines.push(l);
        }

        // NOW wait for exit code
        let status = child.wait().expect("failed to wait for process");
        let code = status.code().unwrap_or(-1);

        (code, lines.clone())
    }

}

