#!/usr/bin/env python3
import os
import subprocess
import sys
from pathlib import Path


# ---------- Helpers ----------

def run(*args, env=None):
    return subprocess.run(
        args,
        env=env,
        check=False,
    ).returncode


def git(*args):
    return run("git", *args)


def git_output(*args):
    return subprocess.check_output(
        ["git", *args],
        text=True,
    ).strip()


def git_path(name):
    return Path(git_output("rev-parse", "--git-path", name))


def current_branch():
    return git_output("branch", "--show-current")


def rebase_in_progress():
    return (
        git_path("rebase-merge").exists()
        or git_path("rebase-apply").exists()
    )


def stash_ref():
    result = subprocess.run(
        ["git", "rev-parse", "-q", "--verify", "refs/stash"],
        capture_output=True,
        text=True,
        check=False,
    )
    return result.stdout.strip() if result.returncode == 0 else None


def capture_pane():
    """Copy the current tmux pane to the clipboard. Never fail."""
    try:
        capture = subprocess.Popen(
            ["tmux", "capture-pane", "-p", "-S", "-"],
            stdout=subprocess.PIPE,
        )
        clipboard = subprocess.Popen(
            ["xclip", "-selection", "clipboard"],
            stdin=capture.stdout,
        )

        if capture.stdout:
            capture.stdout.close()

        capture.wait()
        clipboard.wait()
    except OSError:
        pass


# ---------- Tasks ----------

def github_pull():
    branch = current_branch()

    if not branch:
        print("Cannot pull: detached HEAD", file=sys.stderr)
        return 1

    changes = subprocess.run(
        ["git", "status", "--porcelain", "--untracked-files=no"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout

    saved_stash = None

    if changes:
        before = stash_ref()

        if git("stash", "push") != 0:
            return 1

        saved_stash = stash_ref()

        if saved_stash == before:
            print("Stash was not created", file=sys.stderr)
            return 1

    status = git(
        "pull", "--prune", "--rebase",
        "github", branch,
    )

    if (
        status != 0
        and rebase_in_progress()
        and git("rebase", "--abort") != 0
    ):
        print(
            "Rebase abort failed; stash not applied",
            file=sys.stderr,
        )
        return 1

    if saved_stash:
        apply_status = git("stash", "apply", "--index", saved_stash)

        if apply_status != 0:
            print(
                "Stash apply failed; stash preserved",
                file=sys.stderr,
            )
            if status == 0:
                status = apply_status
        elif stash_ref() == saved_stash:
            if git("stash", "drop", "stash@{0}") != 0:
                print(
                    "Warning: stash drop failed",
                    file=sys.stderr,
                )

    if status == 0:
        status = git("branch")

    return status


def build(release=False):
    """Build, test and lint; capture pane on failure."""
    cargo_flags = ["--release"] if release else []

    test_env = os.environ.copy()
    test_env["RUST_BACKTRACE"] = "1"

    # Clearing the screen is optional.
    run("clear")

    commands = [
        (["git", "branch"], None),
        (["cargo", "build", *cargo_flags], None),
        (["cargo", "test", *cargo_flags], test_env),
        (["cargo", "clippy"], None),
    ]

    for command, env in commands:
        status = run(*command, env=env)

        if status != 0:
            capture_pane()
            return status

    return 0


def build_debug():
    return build(release=False)


def build_release():
    return build(release=True)


def test():
    return run("cargo", "test")


# ---------- Dispatcher ----------

TASKS = {
    "github_pull": github_pull,
    "build-debug": build_debug,
    "build-release": build_release,
    "test": test,
}


def main():
    if len(sys.argv) != 2 or sys.argv[1] not in TASKS:
        print("Usage: python3 build.py <task>")
        print("Available tasks:")
        for name in TASKS:
            print(f"  {name}")
        return 1

    return TASKS[sys.argv[1]]()


if __name__ == "__main__":
    sys.exit(main())

