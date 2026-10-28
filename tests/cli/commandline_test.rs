// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use aiagents::cli::commandline::parse_args_from;

fn parse(args: &[&str]) -> Result<aiagents::cli::commandline::Args, String> {
    parse_args_from(args.iter().map(|arg| (*arg).to_owned()))
}

#[test]
fn inline_messages_are_preserved() {
    let cwd = std::env::current_dir().unwrap();
    let dir = cwd.to_string_lossy();
    let args = parse(&[
        "-l",
        "rust",
        "-t",
        "fix_code",
        "-m",
        "Keep constructor private",
        "--message",
        "Handle Option<Dummy>",
        "-f",
        &dir,
    ])
    .unwrap();
    assert_eq!(
        args.message,
        ["Keep constructor private", "Handle Option<Dummy>"]
    );
    assert_eq!(args.task, "fix_code");
}

#[test]
fn inline_message_requires_nonempty_value() {
    assert!(parse(&["-m"]).unwrap_err().contains("Missing value"));
    assert!(parse(&["-m", ""]).unwrap_err().contains("Empty value"));
}

#[test]
fn inline_messages_work_with_help() {
    let args = parse(&["--message", "Some instruction", "--help"]).unwrap();
    assert_eq!(args.message, ["Some instruction"]);
    assert!(args.help);
}

#[test]
fn defaults_to_fix_code_without_language() {
    let cwd = std::env::current_dir().unwrap();
    let args = parse(&["-f", cwd.to_str().unwrap()]).unwrap();
    assert_eq!(args.task, "fix_code");
    assert_eq!(args.language, None);
}

#[test]
fn explicit_language_and_task_take_precedence() {
    let cwd = std::env::current_dir().unwrap();
    let args = parse(&["-l", "rust", "-t", "build", "-f", cwd.to_str().unwrap()]).unwrap();
    assert_eq!(args.language.as_deref(), Some("rust"));
    assert_eq!(args.task, "build");
}

#[test]
fn buildsystem_language_mapping() {
    use aiagents::generated_languages::Languages;
    use aiagents::runtimetools::buildsystem::Buildsystem;
    assert_eq!(
        Buildsystem::Cargo.inferred_language(),
        Some(Languages::Rust)
    );
    assert_eq!(Buildsystem::Meson.inferred_language(), Some(Languages::Cpp));
    assert_eq!(Buildsystem::Cmake.inferred_language(), Some(Languages::Cpp));
    assert_eq!(
        Buildsystem::Maven.inferred_language(),
        Some(Languages::Java)
    );
    assert_eq!(Buildsystem::Just.inferred_language(), None);
    assert_eq!(Buildsystem::Make.inferred_language(), None);
    assert_eq!(Buildsystem::None.inferred_language(), None);
}

#[test]
fn pathfilter_is_optional_at_parse_time() {
    let args = parse(&[]).unwrap();
    assert!(args.pathfilter.is_empty());
    assert_eq!(args.task, "fix_code");
}

#[test]
fn explicit_pathfilter_is_preserved() {
    let cwd = std::env::current_dir().unwrap();
    let args = parse(&["-f", cwd.to_str().unwrap()]).unwrap();
    assert_eq!(args.pathfilter, vec![cwd]);
}

#[test]
fn help_lists_every_task_once_and_displays_license_and_output_options() {
    use aiagents::cli::commandline::help_text;
    use aiagents::generated_tasks::Tasks;

    let help = help_text();
    assert!(help.starts_with(&format!(
        "aifix {} - Local-first AI agents for software engineering",
        env!("CARGO_PKG_VERSION")
    )));
    assert!(help.contains("AVAILABLE TASKS"));
    assert!(help.contains("Code repair and build"));
    assert!(
        help.contains("Not implemented (task names reserved; no usable task-specific workflow)")
    );
    assert!(help.contains("Review and analysis"));
    assert!(help.contains("Documentation"));
    let task_section = help
        .split("AVAILABLE TASKS\n")
        .nth(1)
        .unwrap()
        .split("TASK-SPECIFIC OPTIONS (after -a)")
        .next()
        .unwrap();
    // Only count task-name rows (four-space indentation), not headings or status prose.
    let listed_tasks: Vec<&str> = task_section
        .lines()
        .filter(|line| line.starts_with("    ") && !line.trim_start().starts_with("Status:"))
        .flat_map(|line| line.split_whitespace())
        .collect();
    for task in Tasks::to_vec_str() {
        let count = listed_tasks.iter().filter(|name| **name == task).count();
        assert_eq!(count, 1, "task {task} appears {count} times in task list");
    }
    assert!(help.contains("-o --output [FILE]"));
    assert!(help.contains("stdout"));
    assert!(help.contains("Copyright (C) 2026 Witold Kaminski"));
    assert!(help.contains("Apache-2.0"));
}

#[test]
fn help_task_groups_have_real_newlines_and_setup_build_is_classified() {
    let help = aiagents::cli::commandline::help_text();
    let task_section = help
        .split("AVAILABLE TASKS\n")
        .nth(1)
        .unwrap()
        .split("TASK-SPECIFIC OPTIONS")
        .next()
        .unwrap();
    assert!(!task_section.contains("\\n"), "literal backslash-n in help");
    assert!(task_section.contains("Code repair and build\n"));
    let not_implemented = task_section
        .split("Not implemented (task names reserved; no usable task-specific workflow)\n")
        .nth(1)
        .expect("missing not-implemented task group")
        .split("\n\n")
        .next()
        .unwrap();
    for task in ["analyze", "gen_code", "transpile_code", "write_test_code"] {
        assert!(
            not_implemented.split_whitespace().any(|name| name == task),
            "{task} missing from not-implemented group"
        );
    }
    assert!(task_section.contains("setup_build"));
    assert!(!task_section.contains("  Other\n"));
}

#[test]
fn tokens_default_to_30000_and_accept_both_switches() {
    assert_eq!(parse(&[]).unwrap().tokens, 30_000);
    assert_eq!(parse(&["-k", "1024"]).unwrap().tokens, 1024);
    assert_eq!(parse(&["--tokens", "65536"]).unwrap().tokens, 65536);
    assert_eq!(parse(&["--tokens", "42", "--help"]).unwrap().tokens, 42);
}

#[test]
fn tokens_reject_missing_invalid_or_zero_values() {
    for args in [
        vec!["-k"],
        vec!["--tokens", "0"],
        vec!["--tokens", "-1"],
        vec!["--tokens", "abc"],
        vec!["--tokens", "4294967296"],
    ] {
        assert!(parse(&args).is_err(), "{args:?}");
    }
}

#[test]
fn help_describes_token_limit() {
    let help = aiagents::cli::commandline::help_text();
    assert!(help.contains("-k, --tokens COUNT"));
    assert!(help.contains("30000"));
}
