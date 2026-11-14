#!/usr/bin/env python3
"""Build an archive from the two native executables, without touching user data."""
import argparse
from pathlib import Path
import tarfile
import zipfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('platform', choices=['linux-x86_64', 'windows-x86_64', 'macos-aarch64'])
    parser.add_argument('output', type=Path)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    suffix = '.exe' if args.platform.startswith('windows') else ''
    files = {
        f'oxidj-cli{suffix}': args.root / f'target/release/oxidj-cli{suffix}',
        f'oxidj-library-ui{suffix}': args.root / f'prototypes/oxidj-library-ui/target/release/oxidj-library-ui{suffix}',
        'README.md': args.root / 'docs/release-readme.md',
        'LICENSE': args.root / 'LICENSE',
    }
    for file in files.values():
        if not file.is_file():
            raise FileNotFoundError(file)
    args.output.mkdir(parents=True, exist_ok=True)
    if suffix:
        with zipfile.ZipFile(args.output / f'oxidj-{args.platform}.zip', 'w', compression=zipfile.ZIP_DEFLATED) as archive:
            for name, file in files.items():
                archive.write(file, name)
    else:
        with tarfile.open(args.output / f'oxidj-{args.platform}.tar.gz', 'w:gz') as archive:
            for name, file in files.items():
                info = archive.gettarinfo(str(file), arcname=name)
                info.mode = 0o755 if name.startswith('oxidj-') else 0o644
                with file.open('rb') as source:
                    archive.addfile(info, source)


if __name__ == '__main__':
    main()
