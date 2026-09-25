import hashlib
import io
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
class ReleaseTests(unittest.TestCase):
    def test_version_and_public_workflow(self):
        version = subprocess.check_output([sys.executable, str(ROOT / 'tools/check-release.py')], text=True).strip()
        self.assertRegex(version, r'^\d+\.\d+\.\d+$')
        result = subprocess.run([sys.executable, str(ROOT / 'tools/check-release.py'), 'v0.0.0'], capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        for workflow in ('test.yml',):
            self.assertEqual((ROOT / 'ci' / workflow).read_bytes(), (ROOT / '.github/workflows' / workflow).read_bytes())

    def test_public_release_cannot_publish(self):
        template = (ROOT / 'ci/public_release.yml').read_text()
        self.assertNotIn('secrets.', template)
        self.assertNotIn('cargo publish', template)
        self.assertNotIn('types: [published]', template)
        private = (ROOT / '.github/workflows/public_release.yml')
        if private.exists():
            text = private.read_text()
            if '  native:' in text:
                self.assertIn("if: github.repository == 'dm-ai-core/dexbot-utils'", text)
            else:
                self.assertEqual(text, template)

    def test_sdk_archives_and_checksums(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            stage = root / 'stage'
            for name in ('include/dexbot.h','include/dexbot/model.hpp','bin/dexbot','lib/libdexbot_model.so','lib/cmake/dexbot/dexbotConfig.cmake', 'lib/cmake/dexbot/dexbotConfigVersion.cmake', 'share/dexbot/LICENSE', 'share/dexbot/LICENSE-URDF', 'share/dexbot/README.md', 'share/dexbot/examples/cpp/CMakeLists.txt', 'share/dexbot/examples/cpp/inspect_model.cpp'):
                path = stage / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('test fixture')
            subprocess.run([sys.executable, str(ROOT / 'tools/package-sdk.py'), str(stage), str(root / 'dist'), 'linux-x86_64'], check=True, capture_output=True)
            archives = list((root / 'dist').glob('*.tar.gz'))
            self.assertEqual(len(archives), 2)
            for archive in archives:
                self.assertEqual(hashlib.sha256(archive.read_bytes()).hexdigest(), archive.with_name(archive.name + '.sha256').read_text().split()[0])
                with tarfile.open(archive) as packed:
                    names = packed.getnames()
                    self.assertTrue(any(name.endswith('/bin/dexbot') for name in names))
                    self.assertTrue(any(name.endswith('/LICENSE-URDF') for name in names))
                    self.assertTrue(all(not name.startswith('/') and '..' not in Path(name).parts for name in names))
            version = subprocess.check_output([sys.executable, str(ROOT / 'tools/check-release.py')], text=True).strip()
            check_command = [sys.executable, str(ROOT / 'tools/check-release-assets.py'), str(root / 'dist'), 'v' + version]
            subprocess.run(check_command, check=True)
            rogue = root / 'dist/private-source.tar.gz'
            rogue.write_bytes(b'private')
            self.assertNotEqual(subprocess.run(check_command, capture_output=True).returncode, 0)
            rogue.unlink()
            sidecar = archives[0].with_name(archives[0].name + '.sha256')
            sidecar.write_text('0' * 64 + '  ' + archives[0].name + '\n')
            self.assertNotEqual(subprocess.run(check_command, capture_output=True).returncode, 0)
            extra = stage / 'private.rs'
            extra.write_text('internal')
            command = [sys.executable, str(ROOT / 'tools/package-sdk.py'), str(stage), str(root / 'bad'), 'linux-x86_64']
            self.assertNotEqual(subprocess.run(command, capture_output=True).returncode, 0)
            extra.unlink()
            (stage / 'include/dexbot.h').unlink()
            result = subprocess.run([sys.executable, str(ROOT / 'tools/package-sdk.py'), str(stage), str(root/'bad'), 'linux-x86_64'], capture_output=True)
            self.assertNotEqual(result.returncode, 0)

    def test_source_export_rejects_private_content(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / 'README.md').write_text('Public API')
            command = [sys.executable, str(ROOT / 'tools/check-public-source.py'), str(root)]
            subprocess.run(command, check=True)
            for name, data in [('secret.env', 'password=abc'), ('Cargo.toml', '[dependencies]\ndexcomm = "0.6"'), ('README.md', 'ghp_' + 'A' * 36)]:
                path = root / name
                path.write_text(data)
                self.assertNotEqual(subprocess.run(command, capture_output=True).returncode, 0)
                path.unlink()

    def test_crate_gate_rejects_private_and_unsafe_entries(self):
        with tempfile.TemporaryDirectory() as temp:
            archive = Path(temp) / 'model.crate'
            base = {'Cargo.toml': '[package]\nname="dexbot-model"\nversion="0.2.0"', 'src/lib.rs': '// public model'}
            command = [sys.executable, str(ROOT / 'tools/check-crate.py'), str(archive), 'dexbot-model', '0.2.0']
            for extra, valid in [({}, True), ({'secret.env': 'secret'}, False),
                                 ({'Cargo.toml': '[dependencies]\ndexcomm="0.6"'}, False),
                                 ({'../escape.rs': 'escape'}, False),
                                 ({'src/leak.rs': 'ghp_' + 'A' * 36}, False)]:
                with tarfile.open(archive, 'w:gz') as packed:
                    for name, data in (base | extra).items():
                        encoded = data.encode()
                        entry = tarfile.TarInfo('dexbot-model-0.2.0/' + name)
                        entry.size = len(encoded)
                        packed.addfile(entry, io.BytesIO(encoded))
                result = subprocess.run(command, capture_output=True)
                self.assertEqual(result.returncode == 0, valid, result.stderr)
