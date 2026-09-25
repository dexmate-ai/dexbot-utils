import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]

class RegenerateTests(unittest.TestCase):
    def test_failures_preserve_all_fixtures_and_success_replaces_them(self):
        for mode in ('build', 'list', 'show', 'empty', 'invalid', 'ok'):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                (root / 'tools').mkdir()
                script = root / 'tools/regenerate-fixtures.sh'
                shutil.copyfile(ROOT / 'tools/regenerate-fixtures.sh', script)
                out = root / 'robots/contracts/resolved-config/v0'
                out.mkdir(parents=True)
                for profile in ('a', 'b'):
                    (out / f'{profile}.json').write_text('original')
                binary = root / 'dexbot'
                binary.write_text('''#!/usr/bin/env bash
set -eu
if [[ "$1" == list ]]; then
  [[ "$MODE" != list ]] || exit 1
  [[ "$MODE" != empty ]] || exit 0
  if [[ "$MODE" == invalid ]]; then echo ../escape; else printf 'a\\nb\\n'; fi
else
  echo '{"new":true}'
  [[ "$MODE" != show || "$2" != b ]] || exit 1
fi
''')
                binary.chmod(0o755)
                cargo = root / 'cargo'
                artifact = json.dumps({'reason':'compiler-artifact','target':{'name':'dexbot'},'executable':str(binary)})
                cargo.write_text('#!/usr/bin/env bash\n[[ "$MODE" != build ]] || exit 1\ncat <<\'JSON\'\n' + artifact + '\nJSON\n')
                cargo.chmod(0o755)
                result = subprocess.run(['bash', str(script)], env={**os.environ, 'MODE':mode, 'PATH':str(root)+os.pathsep+os.environ['PATH']}, capture_output=True)
                self.assertEqual(result.returncode == 0, mode == 'ok', result.stderr)
                for profile in ('a', 'b'):
                    self.assertEqual((out / f'{profile}.json').read_text(), '{"new":true}\n' if mode == 'ok' else 'original')
                self.assertFalse(list(out.glob('.regenerate.*')))
