import pathlib
import subprocess
import sys
import tarfile
import tempfile
import unittest
import zipfile


class ReleasePackaging(unittest.TestCase):
    def test_linux_archive_contains_cli_ui_readme_and_license(self):
        root = pathlib.Path(__file__).resolve().parents[1]
        with tempfile.TemporaryDirectory() as temp:
            base = pathlib.Path(temp)
            cli = base / 'target/release/oxidj-cli'
            ui = base / 'prototypes/oxidj-library-ui/target/release/oxidj-library-ui'
            for binary in (cli, ui):
                binary.parent.mkdir(parents=True, exist_ok=True)
                binary.write_bytes(b'example binary')
            (base / 'LICENSE').write_text('test licence')
            (base / 'docs').mkdir()
            (base / 'docs/release-readme.md').write_text('test readme')
            subprocess.run([sys.executable, str(root / 'scripts/package-release.py'), 'linux-x86_64', str(base / 'out'), '--root', str(base)], check=True)
            with tarfile.open(base / 'out/oxidj-linux-x86_64.tar.gz') as archive:
                members = {m.name: m for m in archive.getmembers()}
                self.assertEqual(set(members), {'oxidj-cli', 'oxidj-library-ui', 'README.md', 'LICENSE'})
                self.assertTrue(members['oxidj-cli'].mode & 0o111)
                self.assertTrue(members['oxidj-library-ui'].mode & 0o111)


    def test_windows_and_macos_archives_and_missing_inputs(self):
        root = pathlib.Path(__file__).resolve().parents[1]
        script = root / 'scripts/package-release.py'
        with tempfile.TemporaryDirectory() as temp:
            base = pathlib.Path(temp)
            cmd = [sys.executable, str(script), 'windows-x86_64', str(base / 'out'), '--root', str(base)]
            self.assertNotEqual(subprocess.run(cmd, capture_output=True).returncode, 0)
            for name in ('oxidj-cli.exe', 'oxidj-library-ui.exe', 'oxidj-cli', 'oxidj-library-ui'):
                parent = base / ('target/release' if 'cli' in name else 'prototypes/oxidj-library-ui/target/release')
                parent.mkdir(parents=True, exist_ok=True)
                (parent / name).write_bytes(b'example binary')
            (base / 'LICENSE').write_text('test licence')
            (base / 'docs').mkdir()
            (base / 'docs/release-readme.md').write_text('test readme')
            subprocess.run(cmd, check=True)
            with zipfile.ZipFile(base / 'out/oxidj-windows-x86_64.zip') as archive:
                self.assertEqual(set(archive.namelist()), {'oxidj-cli.exe', 'oxidj-library-ui.exe', 'README.md', 'LICENSE'})
            subprocess.run([sys.executable, str(script), 'macos-aarch64', str(base / 'out'), '--root', str(base)], check=True)
            with tarfile.open(base / 'out/oxidj-macos-aarch64.tar.gz') as archive:
                self.assertEqual(set(archive.getnames()), {'oxidj-cli', 'oxidj-library-ui', 'README.md', 'LICENSE'})
                self.assertTrue(archive.getmember('oxidj-library-ui').mode & 0o111)


if __name__ == '__main__':
    unittest.main()
