// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use crate::generated_languages::Languages;
use crate::generated_task_argument_help::TASK_ARGUMENT_HELP;
use crate::generated_tasks::Tasks;
use fsscanner::fileentry::FileEntry;
use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug)]
pub struct Args {
    pub language: Option<String>,
    pub task: String,
    pub taskdata: Option<FileEntry>,
    pub subtask: Vec<PathBuf>,
    pub message: Vec<String>,
    pub task_args: Vec<String>,
    pub config: Option<String>,
    pub select: Vec<PathBuf>,
    pub pathfilter: Vec<PathBuf>,
    pub builddir: Option<PathBuf>,
    pub workspace: Option<PathBuf>,
    pub start_service: Option<String>,
    pub provider: Option<String>,
    pub debug: bool,
    pub help: bool,
}

pub fn help() {
    println!(
        r#"aifix [option]+
    -l --lang [optional for language-dependent workflows; detected from build system if omitted]: language: one of: {}
            or provide path to taskdesciption
    -t --task [optional, default = fix_code, multiple possible invalid with -w switch]:
        - first -t selects task: one of: {}
        - following are paths to subtasks that enhance the task description
    -m --message [optional, repeatable]: inline task instructions (quoted text),
        appended to the task context like additional -t files
    -a --args [optional]: all following arguments are passed to the selected task
        task-specific arguments:
{}
    -s --select [optional, multiple possible]: one or more files or dirs,
    -c --config [optional]: load config from path
        - default config will be generated in path if no config availabe
        - default config path is ~/.config/aifix/config.json
    -f --pathfilter [optional, repeatable]: directories aifix can scan and access
        - first -f path: read and write access
        - subsequent -f paths: read-only access
        - if omitted, config.writefilter supplies the first path,
          followed by read-only paths from config.readfilter
        - explicit -f paths replace the configured paths
        - paths must exist; relative paths are resolved from the current directory
        - example: aifix -f src -f tests (src read/write, tests read-only)
    -b --builddir [optional]: override the build output directory
        - default: target (Cargo/Rust, Maven/Java, Just)
        - default: build (CMake, Meson, Make)
        - default: current directory if no build system is detected
    -w --workspace [optional]: running in workspace (mode): path to workspace
        - llm does not load files, all files are loaded at once from current workspace,
        - also task descripton is here and may be named like e.g.: `task.md`
    -d --debug [default = false]: dump debug
    -r --run [optional providername in config] run server; if set other settings will be ignored
    -p --provider [optional providername in config]: select provider for normal tasks
    -h --help [default = false]: dump help
"#,
        Languages::to_vec_str().join(", "),
        Tasks::to_vec_str().join(", "),
        TASK_ARGUMENT_HELP
    );
}

impl Args {
    pub fn is_workspace(&self) -> bool {
        self.workspace.is_some()
    }
}

pub fn parse_args() -> Result<Args, String> {
    parse_args_from(env::args().skip(1))
}

pub fn parse_args_from(arguments: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut args = arguments.into_iter();

    let mut language: Option<String> = None;
    let mut task: Option<String> = None;
    let mut taskdata: Option<FileEntry> = None;
    let mut subtask: Vec<PathBuf> = Vec::new();
    let mut message: Vec<String> = Vec::new();
    let mut task_args: Vec<String> = Vec::new();
    let mut select: Vec<PathBuf> = Vec::new();
    let mut config: Option<String> = None;
    let mut pathfilter: Vec<PathBuf> = Vec::new();
    let mut builddir: Option<PathBuf> = None;
    let mut workspace: Option<PathBuf> = None;
    let mut start_service: Option<String> = None;
    let mut provider: Option<String> = None;
    let mut debug = false;
    let mut help = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-l" | "--lang" => {
                let value = args.next().ok_or("Missing value for --lang")?;
                language = Some(value);
            }
            "-t" | "--task" => {
                let value = args.next().ok_or("Missing value for --task")?;
                if task.is_none() {
                    if let Some((before, after)) = value.split_once(':') {
                        task = Some(before.to_string());
                        let entry = FileEntry::from_str(after);
                        taskdata = Some(entry.unwrap());
                    } else {
                        // Kein Doppelpunkt vorhanden: Der gesamte Wert ist taskdata
                        task = Some(value);
                    }
                } else {
                    let file = match PathBuf::from(value).canonicalize() {
                        Ok(p) => p,
                        Err(_) => return Err("Invalid path for --task".into()),
                    };
                    subtask.push(file);
                }
            }
            "-m" | "--message" => {
                let value = args.next().ok_or("Missing value for --message")?;
                if value.is_empty() {
                    return Err("Empty value for --message".into());
                }
                message.push(value);
            }
            "-a" | "--args" => {
                task_args.extend(args.by_ref());
                break;
            }
            "-s" | "--select" => {
                let value = args.next().ok_or("Missing value for --select")?;
                let file = match PathBuf::from(value).canonicalize() {
                    Ok(p) => p,
                    Err(_) => return Err("Invalid path for --subtask".into()),
                };
                select.push(file);
            }
            "-c" | "--config" => {
                let value = args.next().ok_or("Missing value for --config")?;
                config = Some(value);
            }
            "-f" | "--pathfilter" => {
                let value = args.next().ok_or("Missing value for --pathfilter")?;
                let dir = match PathBuf::from(value).canonicalize() {
                    Ok(p) => p,
                    Err(_) => return Err("Invalid path for --pathfilter".into()),
                };
                pathfilter.push(dir);
            }
            "-b" | "--builddir" => {
                let value = args.next().ok_or("Missing value for --builddir")?;
                builddir = Some(PathBuf::from(value).canonicalize().unwrap());
            }
            "-w" | "--workspace" => {
                let value = args.next().ok_or("Missing value for --workspace")?;
                workspace = Some(PathBuf::from(value).canonicalize().unwrap());
            }

            "-p" | "--provider" => {
                provider = Some(args.next().ok_or("Missing value for --provider")?);
            }
            "-r" | "--run" => {
                let value = args.next().ok_or("Missing value for --run")?;
                start_service = Some(value);
            }

            "-d" | "--debug" => {
                debug = true;
            }
            "-h" | "--help" => {
                help = true;
            }
            unknown => {
                return Err(format!("Unknown argument: {}", unknown));
            }
        }
    }

    // help, start_serice bypasses validation
    if help || start_service.is_some() {
        return Ok(Args {
            language: None,
            task: "fix_code".into(),
            taskdata,
            subtask,
            message,
            task_args,
            config,
            select,
            pathfilter,
            builddir,
            workspace,
            start_service,
            provider,
            debug,
            help,
        });
    }

    let task_value = task.unwrap_or_else(|| "fix_code".to_string());

    // Validate task: must be known OR a valid path
    let known_tasks = Tasks::to_vec_str();

    let is_known = known_tasks.iter().any(|t| t == &task_value);
    let is_path = Path::new(&task_value).exists();

    if !is_known && !is_path {
        return Err(format!(
            "Invalid task '{}'. Must be one of [{}] or a valid file path.",
            task_value,
            known_tasks.join(", ")
        ));
    }

    Ok(Args {
        language,
        task: task_value,
        taskdata,
        subtask,
        message,
        task_args,
        config,
        select,
        pathfilter,
        builddir,
        workspace,
        start_service,
        provider,
        debug,
        help,
    })
}
