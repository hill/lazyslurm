#!/usr/bin/env python3
"""
Update the version in Cargo.toml.

Usage:
  python3 scripts/bump_version.py [patch|minor|major|X.Y.Z]

Prints the new version to stdout on success.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


def read_current_version(cargo_toml: Path) -> tuple[list[str], int, str]:
    text = cargo_toml.read_text(encoding="utf-8").splitlines()
    in_pkg = False
    idx = -1
    cur = None

    for i, line in enumerate(text):
        s = line.strip()
        if s == "[package]":
            in_pkg = True
            continue
        if in_pkg and s.startswith("["):
            break
        if in_pkg:
            m = re.match(r"\s*version\s*=\s*\"([^\"]+)\"", line)
            if m:
                cur = m.group(1)
                idx = i
                break

    if cur is None or idx < 0:
        print("Could not find package.version in Cargo.toml", file=sys.stderr)
        sys.exit(2)

    return text, idx, cur


def bump(v: str, which: str) -> str:
    parts = v.split(".")
    if len(parts) != 3 or not all(p.isdigit() for p in parts):
        print(f"Unsupported version format: {v!r}", file=sys.stderr)
        sys.exit(2)
    major, minor, patch = map(int, parts)
    if which == "major":
        major += 1
        minor = 0
        patch = 0
    elif which == "minor":
        minor += 1
        patch = 0
    else:
        patch += 1
    return f"{major}.{minor}.{patch}"


def main() -> None:
    arg = sys.argv[1] if len(sys.argv) > 1 else "patch"
    cargo_toml = Path("Cargo.toml")

    if not cargo_toml.exists():
        print("Cargo.toml not found in working directory", file=sys.stderr)
        sys.exit(2)

    text, idx, cur = read_current_version(cargo_toml)

    if re.fullmatch(r"\d+\.\d+\.\d+", arg):
        new = arg
    elif arg in {"patch", "minor", "major"}:
        new = bump(cur, arg)
    else:
        print(f"Invalid version argument: {arg}", file=sys.stderr)
        sys.exit(2)

    text[idx] = re.sub(r'(version\s*=\s*\")[^\"]+(\".*)$', rf'\g<1>{new}\2', text[idx])
    cargo_toml.write_text("\n".join(text) + "\n", encoding="utf-8")
    print(new)


if __name__ == "__main__":
    main()

