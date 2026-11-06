#!/usr/bin/env python3
"""Verify exact ZIP inventory and run the extracted binary with a recording host."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import stat
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def smoke(path):
    with tempfile.TemporaryDirectory(prefix='markdown-zip-') as temp, zipfile.ZipFile(path) as archive:
        root = Path(temp) / 'package'
        root.mkdir()
        names = archive.namelist()
        assert len(names) == len(set(name.casefold() for name in names)), 'Duplicate ZIP paths'
        for item in archive.infolist():
            name = PurePosixPath(item.filename)
            assert not name.is_absolute() and '..' not in name.parts and '\\' not in item.filename and ':' not in item.filename
            assert stat.S_ISREG(item.external_attr >> 16), 'Non-regular ZIP entry'
        manifest = json.loads(archive.read('extension.json'))
        inventory = manifest['files']
        assert set(names) == set(inventory) | {'extension.json'}
        for name, digest in inventory.items():
            assert hashlib.sha256(archive.read(name)).hexdigest() == digest, name
        archive.extractall(root)
        binary = root / 'bin' / ('tezbar-' + manifest['slug'] + ('.exe' if os.name == 'nt' else ''))
        assert archive.getinfo(binary.relative_to(root).as_posix()).external_attr >> 16 & 0o111
        binary.chmod(0o755)
        # Compare the checked-in both-target notices with the extracted inventory.
        index = json.loads((root / 'licenses/dependency-license-index.json').read_text(encoding="utf-8"))
        for dependency in index['dependencies']:
            for name, digest in dependency['documents'].items():
                assert inventory[name] == digest and hashlib.sha256((root / name).read_bytes()).hexdigest() == digest
        spec = importlib.util.spec_from_file_location('protocol_smoke', ROOT / 'scripts/protocol-smoke.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.exercise(binary, manifest, package_dir=root)
    print('Extracted ZIP inventory, both-target licenses and real process smoke passed; no installation or desktop effects')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('archive', type=Path)
    smoke(parser.parse_args().archive)
