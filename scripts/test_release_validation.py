"""Execute the workflow's actual validation shell against an isolated fake API.

These are synthetic refusal tests, not evidence of GitHub/native runner success.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class ReleaseValidation(unittest.TestCase):
    def validate(self, scenario='', **overrides):
        workflow = (Path(__file__).resolve().parents[1] / '.github/workflows/release.yml').read_text()
        block = workflow.split('      - name: Validate trusted default-branch dispatch and exact-head CI\n', 1)[1]
        script = block.split('        run: |\n', 1)[1].split('\n  build:', 1)[0]
        script = '\n'.join(line[10:] for line in script.splitlines())
        with tempfile.TemporaryDirectory(prefix='oxidj-validation-') as temporary:
            fake = Path(temporary) / 'gh'
            # gh receives no real token; these responses are synthetic controls.
            fake.write_text('''#!/usr/bin/env python3
import json, os, sys
path = next(arg for arg in sys.argv[1:] if arg.startswith('repos/'))
scenario = os.environ['SCENARIO']
sha = os.environ['GITHUB_SHA']
if scenario == 'api-failure': sys.exit(1)
if path.endswith('/oxidj'): value = {'default_branch': 'master'}
elif path.endswith('/branches/master'):
    value = {'protected': scenario != 'unprotected', 'commit': {'sha': 'wrong' if scenario == 'wrong-sha' else sha}}
elif '/workflows/ci.yml/runs?' in path:
    value = {'workflow_runs': [{'id': 1, 'head_sha': sha, 'head_branch': 'master', 'event': 'push', 'path': '.github/workflows/other.yml' if scenario == 'wrong-workflow' else '.github/workflows/ci.yml', 'status': 'completed', 'conclusion': 'failure' if scenario == 'failed-ci' else 'success'}]}
elif '/runs/1/jobs?' in path:
    print('false' if scenario == 'missing-gate' else 'true'); sys.exit()
elif '/matching-refs/' in path:
    value = [{'ref': 'refs/tags/' + os.environ['TAG']}] if scenario == 'existing-tag' else []
elif '/releases?' in path:
    value = [{'tag_name': os.environ['TAG']}] if scenario == 'existing-release' else []
else: sys.exit(2)
if '--jq' in sys.argv: print(value['default_branch'])
else: print(json.dumps(value))
''')
            fake.chmod(0o755)
            env = dict(os.environ, PATH=temporary + os.pathsep + os.environ['PATH'], SCENARIO=scenario,
                       TAG='v0.1.0-alpha.1', GITHUB_SHA='a' * 40, GITHUB_REPOSITORY='Senci/oxidj',
                       GITHUB_REF='refs/heads/master',
                       WORKFLOW_REF='Senci/oxidj/.github/workflows/release.yml@refs/heads/master',
                       GH_TOKEN='', GITHUB_TOKEN='')
            env.update(overrides)
            return subprocess.run(['bash', '-c', script], env=env, capture_output=True).returncode

    def test_accepts_only_trusted_success(self):
        self.assertEqual(self.validate(), 0)

    def test_refuses_wrong_ref_and_workflow_ref(self):
        self.assertNotEqual(self.validate(GITHUB_REF='refs/tags/v0.1.0-alpha.1'), 0)
        self.assertNotEqual(self.validate(WORKFLOW_REF='Senci/oxidj/.github/workflows/release.yml@refs/heads/untrusted'), 0)

    def test_refuses_invalid_or_injection_tag(self):
        for tag in ('v1.0.0', 'v0.1.0-alpha.1; true', '$(true)', '../bad'):
            with self.subTest(tag=tag):
                self.assertNotEqual(self.validate(TAG=tag), 0)

    def test_refuses_api_error_head_gate_and_existing_version(self):
        for scenario in ('api-failure', 'unprotected', 'wrong-sha', 'wrong-workflow', 'failed-ci', 'missing-gate', 'existing-tag', 'existing-release'):
            with self.subTest(scenario=scenario):
                self.assertNotEqual(self.validate(scenario), 0)


if __name__ == '__main__':
    unittest.main()
