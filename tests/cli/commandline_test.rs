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
        "-l", "rust", "-t", "fix_code",
        "-m", "Keep constructor private",
        "--message", "Handle Option<Dummy>",
        "-f", &dir,
    ]).unwrap();
    assert_eq!(args.message, ["Keep constructor private", "Handle Option<Dummy>"]);
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
