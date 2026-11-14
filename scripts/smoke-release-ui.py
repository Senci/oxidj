#!/usr/bin/env python3
"""Check that an extracted UI stays alive and opens an isolated catalog.

This is a startup smoke, not evidence of interaction or platform qualification.
"""
import argparse
from pathlib import Path
import subprocess
import tempfile
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('executable', type=Path)
    args = parser.parse_args()
    executable = args.executable.resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix='oxidj-release-smoke-') as directory:
        catalog = Path(directory) / 'catalog.sqlite'
        process = subprocess.Popen([str(executable), '--db', str(catalog)])
        try:
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                result = process.poll()
                if result is not None:
                    raise RuntimeError(f'UI exited prematurely with status {result}')
                time.sleep(0.2)
            if not catalog.is_file():
                raise RuntimeError('UI did not create the isolated catalog')
            print('Extracted UI stayed alive for 15 seconds and created a scratch catalog')
        finally:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()


if __name__ == '__main__':
    main()
