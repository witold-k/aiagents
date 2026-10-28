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
    pub tokens: u32,
    pub debug: bool,
    pub help: bool,
}

pub fn help_text() -> String {
    let groups: &[(&str, &[&str])] = &[
        (
            "Code generation and changes [NOT IMPLEMENTED]",
            &["fix_code", "gen_code", "transpile_code", "setup_build"],
        ),
        (
            "Build and verification [NOT IMPLEMENTED]",
            &["build", "write_test_code"],
        ),
        (
            "Review and analysis",
            &[
                "analyze",
                "review_code",
                "review_doc",
                "extract_message_tables",
            ],
        ),
        (
            "Documentation",
            &[
                "release_doc",
                "write_block_doc",
                "write_item_doc",
                "write_module_doc",
            ],
        ),
    ];

    let available = Tasks::to_vec_str();
    let mut tasks = String::new();
    let mut listed = Vec::new();
    for (heading, names) in groups {
        let matches: Vec<_> = names
            .iter()
            .copied()
            .filter(|name| available.contains(name))
            .collect();
        if matches.is_empty() {
            continue;
        }
        tasks.push_str(&format!("  {heading}\n"));
        for pair in matches.chunks(2) {
            tasks.push_str(&format!(
                "    {:<24}{}\n",
                pair[0],
                pair.get(1).copied().unwrap_or("")
            ));
        }
        if *heading == "Review and analysis" && matches.contains(&"extract_message_tables") {
            tasks.push_str("    Status: table extraction is IN DEVELOPMENT / UNSTABLE\n");
        }
        tasks.push('\n');
        listed.extend(matches);
    }
    let other: Vec<_> = available
        .iter()
        .copied()
        .filter(|name| !listed.contains(name))
        .collect();
    if !other.is_empty() {
        tasks.push_str("  Other\n");
        for pair in other.chunks(2) {
            tasks.push_str(&format!(
                "    {:<24}{}\n",
                pair[0],
                pair.get(1).copied().unwrap_or("")
            ));
        }
        tasks.push('\n');
    }

    format!(
        r#"aifix {} - Local-first AI agents for software engineering

USAGE
  aifix [OPTIONS]
  aifix -t TASK [OPTIONS] [-a TASK_ARGS...]

GENERAL OPTIONS
  -h, --help                   Show this help
  -t, --task TASK              Select task (default: fix_code)
                               Repeat -t FILE to add task descriptions
  -l, --lang LANGUAGE          Language (inferred from build system if possible)
                               Available: {}
  -m, --message TEXT           Additional instructions (repeatable)
  -a, --args                   Pass all remaining arguments to selected task
  -s, --select PATH            Select file/directory (repeatable)
  -c, --config FILE            Config file (default: ~/.config/aifix/config.json)
  -f, --pathfilter PATH        First path: read/write; further paths: read-only
                               Overrides configured filters; paths must exist
  -b, --builddir DIR           Build output directory override
                               Default: target, build, or current directory
  -w, --workspace DIR          Use a workspace containing files and task description
  -p, --provider NAME          Select configured LLM provider
  -k, --tokens COUNT           Maximum LLM output tokens (default: 30000)
  -r, --run NAME               Start configured provider server
  -d, --debug                  Enable debug output

AVAILABLE TASKS
{tasks}
TASK-SPECIFIC OPTIONS (after -a)
{}

EXAMPLES
  aifix -t fix_code
  aifix -t release_doc -a -o release-v1.md     # Write release notes to file

Copyright (C) 2026 Witold Kaminski
Licensed under the Apache License, Version 2.0 (Apache-2.0).
"#,
        env!("CARGO_PKG_VERSION"),
        Languages::to_vec_str().join(", "),
        TASK_ARGUMENT_HELP
    )
}

pub fn help() {
    print!("{}", help_text());
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
    let mut tokens: u32 = 30_000;
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
            "-k" | "--tokens" => {
                let value = args.next().ok_or("Missing value for --tokens")?;
                tokens = value
                    .parse::<u32>()
                    .map_err(|_| "Invalid --tokens: expected a positive integer")?;
                if tokens == 0 {
                    return Err("Invalid --tokens: value must be greater than zero".into());
                }
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
            tokens,
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
        tokens,
        debug,
        help,
    })
}
