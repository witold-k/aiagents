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
