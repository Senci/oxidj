#!/usr/bin/env python3
"""Restore the owner's approved 1080p image from the private local reference.

This copies the original image; it does not recreate a live OxiDJ session.
The reference includes owner-authorized tracks and artwork and is not in Git.
Existing output files are never overwritten.
"""
import argparse
import hashlib
import os
from pathlib import Path

REFERENCE = Path.home() / ".local/share/oxidj/private-captures/approved-mix.png"
EXPECTED_SHA256 = "a2a45795984f3544c8b48aedd7f99145246f2cca02ca6082e056ae9e2e5bd12f"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="new PNG path (will not overwrite)")
    args = parser.parse_args()
    try:
        data = REFERENCE.read_bytes()
    except FileNotFoundError:
        parser.exit(1, f"Private reference not found: {REFERENCE}\n")
    if hashlib.sha256(data).hexdigest() != EXPECTED_SHA256:
        parser.exit(1, "Reference differs from the approved frame; refusing to proceed.\n")
    try:
        fd = os.open(args.output, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError:
        parser.exit(1, f"Refusing to overwrite: {args.output}\n")
    try:
        with os.fdopen(fd, "wb") as output:
            output.write(data)
    except BaseException:
        args.output.unlink(missing_ok=True)
        raise
    print(f"Restored the exact approved 1920x1080 frame: {args.output}")
    print("This is the preserved original, not a fresh capture of playing decks.")


if __name__ == "__main__":
    main()
