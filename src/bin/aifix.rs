// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use aiagents::{
    aiagentloop::AIAgentLoop,
    cli::commandline::{help, parse_args},
    config::Config,
    generated_languages::Languages,
    generated_tasks::Tasks,
    generated_workspaces::Workspaces,
    runtimetools::buildsystem::Buildsystem,
    runtimetools::llmcall::LlmCall,
    workflows::select_workflow::WorkflowSelector,
};
use fsscanner::{pathfilter::Pathfilter, pathutils::normalize_path};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};

pub fn run_service(config: &Config, name: &Option<String>) {
    let Some(name) = name else {
        return;
    };

    let Some(provider) = config.get_provider(name) else {
        eprintln!("Provider '{}' not found, available provider:", name);
        for provider in &config.providerlist {
            eprintln!("{}", provider.name);
        }
        return;
    };

    let model = provider.llmmodeldir.join(&provider.model);

    let child = match Command::new(&provider.llmbin)
        .arg("-m")
        .arg(model)
        .args(&provider.llmparam)
        .spawn()
    {
        Ok(child) => Arc::new(Mutex::new(Some(child))),
        Err(err) => {
            eprintln!("Failed to start provider '{}': {}", provider.name, err);
            return;
        }
    };

    #[cfg(feature = "signal-handling")]
    let child_handler = Arc::clone(&child);

    #[cfg(feature = "signal-handling")]
    if let Err(err) = ctrlc::set_handler(move || match child_handler.lock() {
        Ok(mut guard) => {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        Err(err) => {
            eprintln!("warning: failed to lock child process: {err}");
        }
    }) {
        eprintln!("warning: failed to install Ctrl-C handler: {err}");
    }

    match child.lock() {
        Ok(mut guard) => {
            if let Some(child) = guard.as_mut() {
                let _ = child.wait();
            }
        }
        Err(err) => {
            eprintln!("warning: failed to lock child process: {err}");
        }
    }
}

pub fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(err) => {
            eprintln!("Error: {err}");
            help();
            return;
        }
    };

    if args.help {
        help();
        return;
    }

    let is_workspace = args.is_workspace();
    let config = match Config::load_or_create(args.config.clone()) {
        Ok(config) => config,
        Err(err) => {
            eprintln!(
                "cannot load {:?}, check path, may be version conflict. Move or save/delete in this case: {:?}",
                args.config, err
            );
            std::process::exit(1);
        }
    };

    if args.start_service.is_some() {
        run_service(&config, &args.start_service);
        return;
    }

    let filter_paths: Vec<PathBuf> = if args.pathfilter.is_empty() {
        config
            .writefilter
            .iter()
            .chain(config.readfilter.iter())
            .map(PathBuf::from)
            .collect()
    } else {
        args.pathfilter.clone()
    };
    if filter_paths.is_empty() {
        eprintln!(
            "No path filter configured; specify -f/--pathfilter or config.writefilter/config.readfilter"
        );
        std::process::exit(1);
    }
    let filter = Pathfilter::new(filter_paths);

    let src_path = std::env::current_dir().unwrap_or_else(|err| {
        eprintln!("failed to determine current directory: {err}");
        std::process::exit(1);
    });
    let ws_path = args.workspace.clone().unwrap_or_else(|| src_path.clone());
    let bs = {
        let detected = if is_workspace {
            Buildsystem::from_dir(&ws_path)
        } else {
            Buildsystem::from_dir(&src_path)
        };
        if detected.is_none() {
            Buildsystem::from_versioned_project(&src_path)
        } else {
            detected
        }
    };

    let task_name = args.task.clone();
    let task_opt = Tasks::parse(&task_name);
    let task = task_opt.unwrap_or(Tasks::FixCode);

    // Language-independent workflows do not need a build system or language prompt.
    let lang_str = if WorkflowSelector::requires_language(task) {
        let lang_name = args.language.clone().unwrap_or_else(|| {
            bs.inferred_language()
                .unwrap_or_else(|| {
                    eprintln!(
                        "Cannot infer language from build system '{}'; specify -l/--lang",
                        bs.as_str()
                    );
                    std::process::exit(1);
                })
                .as_str()
                .to_string()
        });
        match Languages::parse(&lang_name) {
            Some(lang) => lang.get_prompt().into(),
            None => fs::read_to_string(&lang_name).unwrap_or_else(|err| {
                eprintln!(
                    "Unknown language and failed to load file '{}': {}",
                    lang_name, err
                );
                std::process::exit(1);
            }),
        }
    } else {
        String::new()
    };

    let task_str = match &args.taskdata {
        Some(taskdata) => taskdata.data.clone(),
        None => match task_opt {
            Some(task) => match task.get_prompt().into() {
                Some(prompt) => prompt.to_string(),
                None => fs::read_to_string(&task_name).unwrap_or_else(|err| {
                    eprintln!(
                        "Unknown task and failed to load file '{}': {}",
                        task_name, err
                    );
                    std::process::exit(1);
                }),
            },
            None => {
                eprintln!("No --task set");
                std::process::exit(1);
            }
        },
    };

    let ws_str: String = match args.workspace {
        Some(_) => Workspaces::Sandboxspace.get_prompt().into(),
        None => Workspaces::Srcspace.get_prompt().into(),
    };

    let combined_task = format!("{}\n{}\n{}", lang_str, task_str, ws_str);

    let target_path: PathBuf = {
        match args.builddir {
            Some(bd) => bd,
            None => normalize_path(&bs.get_default_builddir()),
        }
    };

    let provider = match args.provider.as_deref() {
        Some(name) => config.get_provider(name).unwrap_or_else(|| {
            eprintln!("Unknown provider '{name}'");
            std::process::exit(1);
        }),
        None => match config.get_selected_provider() {
            Some(provider) => provider,
            None => return,
        },
    };

    let mut subtask_vec = Vec::<String>::with_capacity(args.subtask.len());
    for subtask in args.subtask {
        let data = fs::read_to_string(&subtask).unwrap_or_else(|err| {
            eprintln!("failed to load subtask file '{:?}': {}", subtask, err);
            std::process::exit(1);
        });
        subtask_vec.push(data);
    }

    // Inline messages use the same persistent subtask context as extra -t files.
    subtask_vec.extend(args.message);

    let llm_call = LlmCall::new(
        &config,
        provider,
        src_path.clone(),
        args.workspace.clone(),
        task,
        combined_task,
        subtask_vec,
        &filter,
        &args.select,
        args.debug,
    );

    let src_path2 = src_path.clone();
    let ws = WorkflowSelector::new(
        &config,
        &llm_call,
        bs,
        &src_path2,
        &ws_path,
        &target_path,
        &args.task_args,
    );
    let workflow = ws.select(task);

    let ailoop = AIAgentLoop::new(
        config.clone(),
        src_path,
        args.workspace,
        &filter,
        workflow,
        args.debug,
    );

    ailoop.run();
}
