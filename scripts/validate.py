#!/usr/bin/env python3
"""Validate source or packaged manifest against the accepted host schema and original metadata."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from package import check_sdk
import jsonschema

ROOT = Path(__file__).resolve().parents[1]


def validate(path):
    manifest = json.loads(path.read_text())
    schema = json.loads((ROOT / 'schemas/extension-v3.schema.json').read_text())
    jsonschema.Draft202012Validator(schema).validate(manifest)
    original = json.loads((ROOT / 'package.json').read_text())
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--no-deps', '--format-version', '1', '--locked'], cwd=ROOT))
    package = next(p for p in metadata['packages'] if Path(p['manifest_path']) == ROOT/'Cargo.toml')
    assert manifest['version'] == package['version']
    check_sdk()
    assert manifest['replaces'] == ['raycast.' + original['name']]
    assert manifest['id'] == 'com.tezbar.native.' + original['name']
    assert manifest['preferences'] == original.get('preferences', [])
    assert manifest['capabilities'] == ['selection.read', 'clipboard.read', 'clipboard.pasteText']
    assert len(manifest['commands']) == len(original['commands']) == 1
    for source, command in zip(original['commands'], manifest['commands']):
        for key, value in source.items():
            assert command[key] == value, key
        assert command['runtime'] == 'rust' and command['concurrency'] == 'serialize'
        assert command['refreshMs'] is None
    assert (ROOT/'resources/assets/command-icon.png').read_bytes() == (ROOT/'assets/command-icon.png').read_bytes()
    from_path = ROOT/'schemas/extension-v3.schema.json'
    assert hashlib.sha256(from_path.read_bytes()).hexdigest() == '6f42f0316aa88097703496e828b11afcc7caecf8477fa7502df903c3ad2a8048'
    for target in ('aarch64-apple-darwin', 'x86_64-pc-windows-msvc'):
        data = json.loads(subprocess.check_output(['cargo', 'metadata', '--format-version', '1', '--locked', '--filter-platform', target], cwd=ROOT))
        nodes = {n['id']: n for n in data['resolve']['nodes']}
        selected, pending = set(), [data['resolve']['root']]
        while pending:
            ident = pending.pop()
            if ident in selected:
                continue
            selected.add(ident)
            pending.extend(d['pkg'] for d in nodes[ident]['deps'] if any(k['kind'] in (None, 'build') for k in d['dep_kinds']))
        for package in data['packages']:
            if package['id'] not in selected:
                continue
            assert package['name'] not in ('tauri', 'tezbar'), package['name']
            version = package.get('rust_version')
            assert version and tuple(map(int, version.split('.'))) <= (1, 85, 0), (package['name'], version)
            if package['source'] is None:
                assert Path(package['manifest_path']).is_relative_to(ROOT), package['manifest_path']
        print(target, 'locked runtime/build dependency MSRV <= 1.85; production graph has no host/Tauri dependency')
    print('Schema 3 + original commands/preferences/icon + immutable SDK policy validated')


if __name__ == '__main__':
    p = argparse.ArgumentParser()
    p.add_argument('manifest', type=Path, nargs='?', default=ROOT / 'extension.json')
    validate(p.parse_args().manifest)
