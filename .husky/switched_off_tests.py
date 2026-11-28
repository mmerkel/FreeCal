"""Every switched-off test names the open ticket that switches it back on.

Decision 1 of .scratch/freecal-v1/redesign-plan.md; ticket 173.

Searches core/, src-tauri/ and src/ (build output excepted) and accepts:

    Rust:      #[ignore = "until ticket NN: <why>"]
    Frontend:  // until ticket NN: <why>
               it.skip(...)          (or describe.skip, test.skip)

The frontend note is a comment on the line right before the skip, so
switching the test back on deletes two lines and leaves its name as it was.
A frontend note with no skip on the next line is refused too: it is left
over from a test switched back on.
Ticket NN has a file .scratch/freecal-v1/issues/NN-*.md whose first Status
line is neither done nor wontfix.

Usage: switched_off_tests.py [repo-root]. Prints file:line for each refusal.
"""

import glob
import os
import re
import sys

ROOT = sys.argv[1] if len(sys.argv) > 1 else "."
SEARCHED = ["core", "src-tauri", "src"]
SKIPPED_DIRS = {"target", "node_modules"}
ISSUES = os.path.join(ROOT, ".scratch", "freecal-v1", "issues")

RUST_SWITCH = re.compile(r"#\[ignore\b")
RUST_NOTE = re.compile(r'#\[ignore = "until ticket (\d+): \S[^"]*"\]')
FRONT_SWITCH = re.compile(r"\b(?:it|describe|test)\.skip\b")
FRONT_NOTE = re.compile(r"^\s*// until ticket (\d+): \S.*$")
FORM = {
    ".rs": '#[ignore = "until ticket NN: <why>"]',
    "front": "a comment `// until ticket NN: <why>` on the line before",
}


def ticket_problem(num):
    files = glob.glob(os.path.join(ISSUES, f"{num}-*.md"))
    if not files:
        return f"no ticket {num} in .scratch/freecal-v1/issues/"
    with open(files[0]) as f:
        status = re.search(r"\*\*Status:\*\*\s*(\S+)", f.read())
    if status and status.group(1) in ("done", "wontfix"):
        return f"ticket {num} is {status.group(1)}"
    return None


def problems(path, lines):
    rust = path.endswith(".rs")
    for i, line in enumerate(lines):
        if not rust and FRONT_NOTE.match(line) and not (i + 1 < len(lines) and FRONT_SWITCH.search(lines[i + 1])):
            yield i + 1, "a note `// until ticket NN:` needs a skipped test on the next line"
        if rust and RUST_SWITCH.search(line):
            note = RUST_NOTE.search(line)
        elif not rust and FRONT_SWITCH.search(line):
            note = FRONT_NOTE.match(lines[i - 1]) if i > 0 else None
        else:
            continue
        if not note:
            yield i + 1, f"a switched-off test needs {FORM['.rs' if rust else 'front']}"
        elif problem := ticket_problem(note.group(1)):
            yield i + 1, problem


def files():
    for top in SEARCHED:
        for dirpath, dirnames, filenames in os.walk(os.path.join(ROOT, top)):
            dirnames[:] = sorted(d for d in dirnames if d not in SKIPPED_DIRS and not d.startswith("."))
            for name in sorted(filenames):
                if name.endswith((".rs", ".ts", ".js", ".svelte")):
                    yield os.path.join(dirpath, name)


refused = False
for path in files():
    with open(path, encoding="utf-8", errors="replace") as f:
        lines = f.read().splitlines()
    for line, problem in problems(path, lines):
        print(f"{os.path.relpath(path, ROOT)}:{line}: {problem}")
        refused = True
sys.exit(1 if refused else 0)
