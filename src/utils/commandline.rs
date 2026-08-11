// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use fsscanner::fileentry::FileEntry;
use crate::generated_tasks::Tasks;
use crate::generated_languages::Languages;

#[derive(Debug)]
pub struct Args {
    pub language: String,
    pub task: String,
    pub taskdata: Option<FileEntry>,
    pub subtask: Vec<PathBuf>,
    pub config: Option<String>,
    pub select: Vec<PathBuf>,
    pub pathfilter: Vec<PathBuf>,
    pub builddir: Option<PathBuf>,
    pub workspace: Option<PathBuf>,
    pub start_service: Option<String>,
    pub debug: bool,
    pub help: bool,
}


pub fn help() {
    println!(r#"aifix [option]+
-l --lang [required, invalid with -w switch]: select task: one of: {}
        or provide path to taskdesciption
-t --task [required, multiple possible invalid with -w switch]:
    - first -t selects task: one of: {}
    - following are paths to subtasks that enhance the task description
-s --select [optional, multiple possible]: one or more files or dirs,
    - if task needs a file to operate and none is given a random file will be choosen
-c --config [optional]: load config from path
    - default config will be generated in path if no config availabe
    - default config path is ~/.config/aifix/config.json
-f --pathfilter [multiple, required at least once]: directory list
-b --builddir [default = target(rust) or build(other)]: set builddir
-w --workspace [optional]: running in workspace (mode): path to workspace
    - llm does not load files, all files are loaded at once from current workspace,
    - also task descripton is here and may be named like e.g.: `task.md`
-d --debug [default = false]: dump debug
-r --run [optional providername in config] run server; if set other settings will be ignored
-h --help [default = false]: dump help
"#
        , Languages::to_vec_str().join(", ")
        , Tasks::to_vec_str().join(", ")
    );
}

impl Args {
    pub fn is_workspace(&self) -> bool {
        self.workspace.is_some()
    }
}

pub fn parse_args() -> Result<Args, String> {
    let mut args = env::args().skip(1);

    let mut language: Option<String> = None;
    let mut task: Option<String> = None;
    let mut taskdata: Option<FileEntry> = None;
    let mut subtask: Vec<PathBuf> = Vec::new();
    let mut select: Vec<PathBuf> = Vec::new();
    let mut config: Option<String> = None;
    let mut pathfilter: Vec<PathBuf> = Vec::new();
    let mut builddir: Option<PathBuf> = None;
    let mut workspace: Option<PathBuf> = None;
    let mut start_service: Option<String> = None;
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
                }
                else {
                    let file = match PathBuf::from(value).canonicalize() {
                        Ok(p) => p,
                        Err(_) => return Err("Invalid path for --task".into()),
                    };
                    subtask.push(file);
               }
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
            language: "".into(),
            task: "".into(),
            taskdata,
            subtask,
            config,
            select,
            pathfilter,
            builddir,
            workspace,
            start_service,
            debug,
            help,
        });
    }

    // Required: --task
    let lang_value = language.ok_or("Missing required argument --task")?;

    // Required: --task
    let task_value = task.ok_or("Missing required argument --task")?;

    // Required: at least one --pathfilter
    if pathfilter.is_empty() {
        return Err("Missing required argument --pathfilter (must be provided at least once)".into());
    }

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
        language: lang_value,
        task: task_value,
        taskdata,
        subtask,
        config,
        select,
        pathfilter,
        builddir,
        workspace,
        start_service,
        debug,
        help,
    })
}

