// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::fs;
use std::process::Command;
use fsscanner::{
    pathfilter::Pathfilter,
    pathutils::normalize_path,
};
use aiagents::{
    aiagentloop::AIAgentLoop,
    utils::commandline::{parse_args, help},
    utils::config::Config,
    workflows::{
        buildsystem::Buildsystem,
        select_workflow::WorkflowSelector,
    },
    generated_languages::Languages,
    generated_tasks::Tasks,
    generated_workspaces::Workspaces,
};

pub fn run_service(config: &Config, name: &Option<String>) {
    let Some(name) = name else {
        return;
    };

    let Some(provider) = config.get_provider(name) else {
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

    let child_handler = Arc::clone(&child);

    if let Err(err) = ctrlc::set_handler(move || {
        match child_handler.lock() {
            Ok(mut guard) => {
                if let Some(mut child) = guard.take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
            Err(err) => {
                eprintln!("warning: failed to lock child process: {err}");
            }
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
        Err(_) => { help(); return; }
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

    let filter = Pathfilter::new(
        args.pathfilter.iter().map(PathBuf::from).collect::<Vec<_>>()
    );

    let task_name = args.task.clone();
    let lang_name = args.language.clone();
    let task_opt = Tasks::parse(&task_name);
    let lang_opt = Languages::parse(&lang_name);

    let lang_str = match lang_opt {
        Some(lang) => lang.get_prompt().into(),
        None => fs::read_to_string(&lang_name).unwrap_or_else(|err| {
            eprintln!("Unknown language and failed to load file '{}': {}", lang_name, err);
            std::process::exit(1);
        }),
    };

    let task_str = match &args.taskdata {
        Some(taskdata) => taskdata.data.clone(),
        None => {
            match task_opt {
                Some(task) => match task.get_prompt().into() {
                    Some(prompt) => prompt.to_string(),
                    None => fs::read_to_string(&task_name).unwrap_or_else(|err| {
                        eprintln!("Unknown task and failed to load file '{}': {}", task_name, err);
                        std::process::exit(1);
                    }),
                },
                None => {
                    eprintln!("No --task set");
                    std::process::exit(1);
                }
            }
        }
    };

    let ws_str: String = match args.workspace {
        Some(_) => Workspaces::Sandboxspace.get_prompt().into(),
        None    => Workspaces::Srcspace.get_prompt().into(),
    };

    let combined_task = format!("{}\n{}\n{}", lang_str, task_str, ws_str);

    let task = task_opt.unwrap_or(Tasks::FixCode);
    let src_path: PathBuf = ".".into();
    let ws_path = {
        if is_workspace {
            args.workspace.clone().unwrap()
        }
        else {
            src_path.clone()
        }
    };

    let bs = {
        if is_workspace {
            let bs = Buildsystem::from_dir(&ws_path);
            if bs.is_none() { Buildsystem::from_versioned_project(&src_path) } else { bs }
        }
        else {
            let bs = Buildsystem::from_current_dir();
            if bs.is_none() { Buildsystem::from_versioned_project(&src_path) } else { bs }
        }
    };
    let target_path: PathBuf = {
        match args.builddir {
            Some(bd) => bd,
            None => {
                normalize_path(&bs.get_default_builddir())
            }
        }
    };

    let src_path2 = src_path.clone();
    let ws = WorkflowSelector::new(&bs, &src_path2, &ws_path, &target_path);
    let wf = ws.select(task);

    let mut subtask_vec = Vec::<String>::with_capacity(args.subtask.len());
    for subtask in args.subtask {
        let data = fs::read_to_string(&subtask).unwrap_or_else(|err| {
            eprintln!("failed to load subtask file '{:?}': {}", subtask, err);
            std::process::exit(1);
        });
        subtask_vec.push(data);
    };
    let loop_ = AIAgentLoop::new(
        config,
        src_path,
        args.workspace,
        task,
        combined_task,
        subtask_vec,
        &filter,
        &args.select,
        wf,
        args.debug,
    );

    loop_.run();
}
