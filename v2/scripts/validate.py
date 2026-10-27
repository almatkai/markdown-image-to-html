#!/usr/bin/env python3
"""Full pinned JSON Schema validation plus metadata/provenance/publication checks.
Requires jsonschema (tested with 4.25.1); has no host or network dependency.
"""
import hashlib
import json
from pathlib import Path
import subprocess
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parent.parent

def load(path):
    return json.loads((ROOT / path).read_text())

schema = load('tests/fixtures/beta/extension-v2.schema.json')
Draft202012Validator.check_schema(schema)
manifest = load('manifest.json')
Draft202012Validator(schema).validate(manifest)
source = load('tests/fixtures/v1/package.json')
upstream = load('tests/fixtures/v1/raycast-package.json')
assert manifest['id'] == 'com.tezbar.native.markdown-image-to-html'
assert manifest['replaces'] == ['raycast.markdown-image-to-html']
assert manifest['slug'] == source['name'] == upstream['name'] == 'markdown-image-to-html'
assert manifest['name'] == source['title'] == upstream['title']
assert manifest['version'] == '0.1.0'
assert manifest['runtime'] == 'rust'
assert manifest['platforms'] == ['macOS', 'Windows']
assert manifest['icon'] == '/extensions/icons/markdown-image-to-html.png'
assert manifest['description'].startswith(source['description'])
assert upstream['author'] == 'qeude'
assert upstream['contributors'] == ['daniellacoste']
assert all(s in manifest['description'] for s in ['qeude', 'daniellacoste'])
assert len(manifest['commands']) == 1
command = manifest['commands'][0]
assert command == dict(source['commands'][0], runtime='rust')
assert command['mode'] == 'no-view'
assert command['arguments'] == upstream['commands'][0]['arguments']
assert command['preferences'] == upstream['commands'][0]['preferences']
assert manifest['verification'] == [dict(platform=p, state='not-run', note='Real selected text, clipboard, focus and paste QA has not run.') for p in ['macOS', 'Windows']]
rows = load('tests/fixtures/provenance.json')
files = {r['file'] for r in rows}
assert len(rows) == len(files)
assert {str(p.relative_to(ROOT)) for d in ['tests/fixtures/beta','tests/fixtures/v1'] for p in (ROOT/d).rglob('*') if p.is_file()} <= files
assert {'assets/command-icon.png','licenses/Apache-2.0.txt','licenses/own-V1-LICENSE','licenses/Raycast-MIT.txt'} <= files
for row in rows:
    data = (ROOT/row['file']).read_bytes()
    assert hashlib.sha256(data).hexdigest() == row['sha256'], row['file']
    assert row['license'] in ['MIT', 'Apache-2.0']
    assert len(row['commit']) == 40
    if 'bytes' in row:
        assert len(data) == row['bytes']
assert len((ROOT/'assets/command-icon.png').read_bytes()) == 63310
assert 'license = "MIT AND Apache-2.0"' in (ROOT/'Cargo.toml').read_text()
# In a Git checkout also ensure every deliverable is visible to publication.
# Extracted Cargo packages have no enclosing repository and skip this part.
repo = subprocess.run(['git','rev-parse','--show-toplevel'],cwd=ROOT,capture_output=True,text=True)
if repo.returncode == 0 and (Path(repo.stdout.strip())/'v2').resolve() == ROOT:
    paths = ['Cargo.lock','Cargo.toml','manifest.json','README.md','INTEGRATION.md','NOTICE.md','VALIDATION.md']
    paths += [str(p.relative_to(ROOT)) for d in ['src','tests','scripts','examples','licenses','assets'] for p in (ROOT/d).rglob('*') if p.is_file()]
    for name in paths:
        result = subprocess.run(['git','check-ignore','--quiet',str(ROOT/name)],cwd=ROOT)
        assert result.returncode == 1, f'Git ignores required publication file: {name}'
print(f'Full Draft 2020-12 schema, metadata, {len(rows)} provenance hashes, icon and publication inventory: PASS')
