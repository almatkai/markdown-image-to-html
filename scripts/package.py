#!/usr/bin/env python3
"""Build the root-layout ZIP and SHA inventories from an already built binary. No uploads."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import struct
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
TARGETS = {'aarch64-apple-darwin': False, 'x86_64-pc-windows-msvc': True}
SDK_REV = 'aaec571673ab64a85a136cf6bd054ef660c4d93c'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_sdk():
    source = (ROOT / 'Cargo.toml').read_text()
    match = re.search(r'^tezbar-extension-sdk\s*=\s*\{([^}\n]*)\}', source, re.M)
    fields = match.group(1) if match else ''
    revision = re.search(r'\brev\s*=\s*"([0-9a-f]{40})"', fields)
    if (not revision or revision.group(1) != SDK_REV
            or re.search(r'\b(path|branch|tag)\s*=', fields)
            or not re.search(r'\bgit\s*=\s*"https://github.com/almatkai/tezbar-extension-sdk-rust.git"', fields)):
        raise SystemExit('Release blocked: use the approved immutable SDK git revision')
    entry = re.search(r'\[\[package\]\]\nname = "tezbar-extension-sdk"(.*?)(?=\[\[package\]\]|\Z)',
                      (ROOT / 'Cargo.lock').read_text(), re.S)
    if not entry or not re.search(r'source = "git\+https://github.com/almatkai/tezbar-extension-sdk-rust.git\?rev=' + SDK_REV + '#' + SDK_REV + '"', entry.group(1)):
        raise SystemExit('Release blocked: Cargo.lock must pin the same immutable SDK revision')


def check_target(binary, target):
    data = binary.read_bytes()
    if TARGETS[target]:
        assert os.name == 'nt', 'Windows packages must be verified by running their real executable on Windows'
        assert data[:2] == b'MZ' and len(data) >= 64, 'Expected Windows executable'
        offset = struct.unpack_from('<I', data, 60)[0]
        assert data[offset:offset + 4] == b'PE\0\0' and struct.unpack_from('<H', data, offset + 4)[0] == 0x8664
    else:
        assert os.name != 'nt', 'Run macOS executable verification on macOS'
        assert struct.unpack_from('<II', data) == (0xfeedfacf, 0x100000c), 'Expected thin arm64 Mach-O'


def build(target, binary, output):
    check_sdk()
    assert not binary.is_symlink(), 'Binary symlinks cannot be packaged'
    check_target(binary, target)
    subprocess.run(['python3', str(ROOT / 'scripts/vendor-licenses.py'), '--check'], cwd=ROOT, check=True)
    manifest = json.loads((ROOT / 'extension.json').read_text())
    slug = manifest['slug']
    executable = 'bin/tezbar-' + slug + ('.exe' if TARGETS[target] else '')
    info = json.loads(subprocess.check_output([str(binary.resolve()), '--protocol-info'], timeout=10))
    assert (info['id'], info['version'], info['protocol']) == (manifest['id'], manifest['version'], 1), 'Binary identity mismatch'
    platform = 'Windows' if TARGETS[target] else 'macOS'
    expected = [c['name'] for c in manifest['commands'] if c['runtime'] == 'rust' and platform in c['platforms']]
    assert set(info['commands']) == set(expected), 'Binary/manifest platform coverage mismatch'
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as temp:
        stage = Path(temp)
        (stage / 'bin').mkdir()
        shutil.copy2(binary, stage / executable)
        for name in ('resources', 'licenses'):
            shutil.copytree(ROOT / name, stage / name, symlinks=True)
        for name in ('LICENSE', 'NOTICE'):
            assert not (ROOT / name).is_symlink()
            shutil.copy2(ROOT / name, stage / name)
        files = {}
        expanded = 0
        for path in sorted(stage.rglob('*')):
            assert not path.is_symlink(), 'Symlinks cannot be packaged'
            if path.is_file():
                files[path.relative_to(stage).as_posix()] = digest(path)
                expanded += path.stat().st_size
        assert len(files) + 1 <= 4096 and expanded < 512 * 1024 * 1024, 'Host expanded archive limits exceeded'
        manifest['files'] = files
        # The trusted release manifest supplies published URL/archive digests after review;
        # an archive cannot contain its own SHA256. No invented release URLs or hashes.
        manifest['artifacts'] = {}
        (stage / 'extension.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
        archive = output / f'{slug}-{target}.zip'
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as result:
            for path in sorted(stage.rglob('*')):
                if path.is_file():
                    name = path.relative_to(stage).as_posix()
                    entry = zipfile.ZipInfo(name, date_time=(2026, 1, 1, 0, 0, 0))
                    entry.create_system = 3
                    entry.external_attr = (stat.S_IFREG | (0o755 if name == executable else 0o644)) << 16
                    entry.compress_type = zipfile.ZIP_DEFLATED
                    result.writestr(entry, path.read_bytes())
        assert archive.stat().st_size <= 128 * 1024 * 1024, 'Host archive limit exceeded'
        with zipfile.ZipFile(archive) as result:
            assert set(result.namelist()) == set(files) | {'extension.json'}
            assert len(result.namelist()) == len({name.casefold() for name in result.namelist()})
            for name, sha in files.items():
                assert hashlib.sha256(result.read(name)).hexdigest() == sha, name
        artifact = {'target': target, 'sha256': digest(archive), 'executable': executable, 'executableSha256': files[executable]}
        (output / f'{slug}-{target}.artifact.json').write_text(json.dumps(artifact, indent=2) + '\n')
        (output / f'{slug}-{target}.files.json').write_text(json.dumps(files, indent=2) + '\n')
        (output / f'tezbar-{slug}-{target}.sha256').write_text(f"{artifact['executableSha256']}  {executable}\n")
        (output / f'{archive.name}.sha256').write_text(f"{artifact['sha256']}  {archive.name}\n")
        print(archive)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--target', required=True, choices=TARGETS)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--output', type=Path, default=ROOT / 'dist')
    args = parser.parse_args()
    build(args.target, args.binary, args.output)
