#!/usr/bin/env python3
"""Extract pinned official Argyll icclib sources into a fresh directory."""
import argparse
import hashlib
from pathlib import Path
import zipfile

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('archive', type=Path)
p.add_argument('destination', type=Path)
a = p.parse_args()
expected = 'f8576ce5589fd15620abb73ff049ea31f55ddbd1bba6d1ffa87452658e7bc85f'
if hashlib.sha256(a.archive.read_bytes()).hexdigest() != expected:
    raise SystemExit('Argyll 3.5.0 archive SHA-256 mismatch')
a.destination.mkdir(parents=True, exist_ok=False)
prefix = 'Argyll_V3.5.0/icc/'
with zipfile.ZipFile(a.archive) as archive:
    count = 0
    for member in archive.infolist():
        if not member.filename.startswith(prefix) or member.is_dir():
            continue
        name = member.filename[len(prefix):]
        if not name or Path(name).is_absolute() or '..' in Path(name).parts:
            raise ValueError('unsafe archive path')
        target = a.destination/name
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('xb') as out:
            out.write(archive.read(member))
        count += 1
print(f'extracted {count} ICC source/reference files')
