"""Serialized research checkpoint: stage named paths, log progress, commit, push.

Safe to call from concurrently running agents: an exclusive lock file
serializes git index operations and PROGRESS.md edits. Only paths under
research/ may be staged unless --allow-outside is given. Refuses to run on any
branch other than dev.

Usage (Windows or WSL Python >= 3.10):
  python research/orchestration/checkpoint.py \
      --subject "research: add chunking experiment specs" \
      --body "Why/what paragraph..." \
      --note "Pre-registered EXP-CDC-* specs." \
      research/experiments/EXP-CDC-001 research/experiments/EXP-CDC-002

Exit codes: 0 committed (or nothing to commit with --allow-empty-skip), 2 usage
or policy refusal, 3 lock timeout, 4 git failure.
"""

import argparse
import datetime as _dt
import os
import pathlib
import subprocess
import sys
import time

REPO = pathlib.Path(__file__).resolve().parents[2]
LOCK = REPO / ".git" / "research-checkpoint.lock"
PROGRESS = REPO / "research" / "PROGRESS.md"
TRAILER = "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
CHANGELOG_HEADING = "## Change log"


def git(*args, check=True, capture=True):
    result = subprocess.run(
        ["git", "-C", str(REPO), *args],
        capture_output=capture,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if check and result.returncode != 0:
        sys.stderr.write(f"git {' '.join(args)} failed ({result.returncode}):\n{result.stdout}\n{result.stderr}\n")
        sys.exit(4)
    return result


def acquire_lock(timeout_s):
    deadline = time.monotonic() + timeout_s
    while True:
        try:
            fd = os.open(LOCK, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
            os.write(fd, f"{os.getpid()} {_dt.datetime.now(_dt.timezone.utc).isoformat()}\n".encode())
            os.close(fd)
            return
        except FileExistsError:
            # Break locks older than 30 minutes (crashed holder).
            try:
                if time.time() - LOCK.stat().st_mtime > 1800:
                    LOCK.unlink(missing_ok=True)
                    continue
            except FileNotFoundError:
                continue
            if time.monotonic() > deadline:
                sys.stderr.write(f"timed out waiting for {LOCK}\n")
                sys.exit(3)
            time.sleep(2)


def append_progress_note(note):
    text = PROGRESS.read_text(encoding="utf-8")
    stamp = _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H:%MZ")
    entry = f"- {stamp} — {note.strip()}\n"
    if CHANGELOG_HEADING not in text:
        text = text.rstrip("\n") + f"\n\n{CHANGELOG_HEADING}\n\n"
    if not text.endswith("\n"):
        text += "\n"
    text += entry
    PROGRESS.write_text(text, encoding="utf-8", newline="\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--subject", required=True)
    parser.add_argument("--body", default="")
    parser.add_argument("--note", default="", help="one-line PROGRESS.md change-log entry")
    parser.add_argument(
        "--co-author",
        default=TRAILER,
        help="full Co-Authored-By trailer line for the model that produced the change",
    )
    parser.add_argument("--no-push", action="store_true")
    parser.add_argument("--allow-outside", action="store_true")
    parser.add_argument("--allow-empty-skip", action="store_true", help="exit 0 if nothing is staged")
    parser.add_argument("--lock-timeout", type=int, default=900)
    parser.add_argument("paths", nargs="+")
    args = parser.parse_args()

    if len(args.subject) > 72:
        sys.stderr.write("subject longer than 72 characters\n")
        sys.exit(2)
    if not args.co_author.strip().startswith("Co-Authored-By: "):
        sys.stderr.write("--co-author must be a full 'Co-Authored-By: Name <email>' line\n")
        sys.exit(2)
    for raw in args.paths:
        native = pathlib.Path(raw)
        rel = pathlib.PurePosixPath(native.as_posix())
        # Check absoluteness with native semantics: "D:/repo/x" is absolute on Windows
        # but not as a PurePosixPath.
        if native.is_absolute() or rel.is_absolute() or ".." in rel.parts:
            resolved = native.resolve()
            try:
                rel = pathlib.PurePosixPath(resolved.relative_to(REPO).as_posix())
            except ValueError:
                sys.stderr.write(f"path outside repository: {raw}\n")
                sys.exit(2)
        if not args.allow_outside and (not rel.parts or rel.parts[0] != "research"):
            sys.stderr.write(f"refusing to stage non-research path without --allow-outside: {raw}\n")
            sys.exit(2)

    acquire_lock(args.lock_timeout)
    try:
        branch = git("rev-parse", "--abbrev-ref", "HEAD").stdout.strip()
        if branch != "dev":
            sys.stderr.write(f"refusing to commit on branch {branch!r}; research commits belong on dev\n")
            sys.exit(2)
        git("add", "--", *args.paths)
        if args.note:
            append_progress_note(args.note)
            git("add", "--", str(PROGRESS.relative_to(REPO)))
        staged = git("diff", "--cached", "--name-only").stdout.split()
        if not staged:
            if args.allow_empty_skip:
                print("nothing staged; skipped")
                return
            sys.stderr.write("nothing staged\n")
            sys.exit(2)
        message = args.subject.strip() + "\n\n"
        if args.body.strip():
            message += args.body.strip() + "\n\n"
        message += args.co_author.strip() + "\n"
        commit = subprocess.run(
            ["git", "-C", str(REPO), "commit", "-q", "-F", "-"],
            input=message,
            text=True,
            encoding="utf-8",
            capture_output=True,
        )
        if commit.returncode != 0:
            sys.stderr.write(commit.stdout + commit.stderr)
            sys.exit(4)
        sha = git("rev-parse", "--short", "HEAD").stdout.strip()
        print(f"committed {sha} ({len(staged)} files): {args.subject}")
        if not args.no_push:
            for attempt in range(3):
                push = git("push", "origin", "dev", check=False)
                if push.returncode == 0:
                    print(f"pushed {sha} to origin/dev")
                    break
                time.sleep(5 * (attempt + 1))
            else:
                sys.stderr.write(f"push failed (commit {sha} kept locally):\n{push.stderr}\n")
                sys.exit(4)
    finally:
        LOCK.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
