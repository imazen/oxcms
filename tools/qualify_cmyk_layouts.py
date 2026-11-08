#!/usr/bin/env python3
"""Run prepared/strided CMYK row checks on a pinned profile manifest."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

p = argparse.ArgumentParser(description=__doc__)
for name in ['matrix-report', 'profiles-root', 'evaluator', 'c-oracle', 'output']:
    p.add_argument('--'+name, required=True, type=Path)
a = p.parse_args()
a.output.mkdir(parents=True, exist_ok=False)
manifest = json.loads(a.matrix_report.read_text())
report = {'pixels_per_cell': 514, 'width': 257, 'rows': 2,
          'source_prefix_bytes': 'one sample, backed by aligned u32 storage', 'source_padding_pixels': 3,
          'reference': 'C LCMS, relative intent, BPC on, native U8/U16/F32 execution',
          'cases': [], 'failures': []}
root = a.profiles_root.resolve()
for profile in manifest['profiles']:
    source = root/profile['path']
    assert hashlib.sha256(source.read_bytes()).hexdigest() == profile['sha256']
    for target in [root/'crates/argyll-sys/sRGB.icm', root/'external/argyllcms/icc/DisplayP3.icm']:
        output = a.output/(profile['sha256'][:16]+'-'+target.stem)
        result = subprocess.run([str(a.evaluator.resolve()), str(source), str(target),
                                 str(a.c_oracle.resolve()), str(output)], capture_output=True)
        cell = {'profile': profile['path'], 'source_sha256': profile['sha256'],
                'target': target.name, 'target_sha256': hashlib.sha256(target.read_bytes()).hexdigest(),
                'exit': result.returncode, 'stdout': result.stdout.decode(errors='replace'),
                'stderr': result.stderr.decode(errors='replace')}
        report['cases'].append(cell)
        if result.returncode:
            report['failures'].append(cell)
        (a.output/'report.json').write_text(json.dumps(report, indent=2)+'\n')
        print(profile['path'], target.name, result.returncode, flush=True)
report['accepted_layouts'] = sum(c['stdout'].count('|ok|') for c in report['cases'])
report['declined_layouts'] = sum(c['stdout'].count('|declined') for c in report['cases'])
(a.output/'report.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps({k: report[k] for k in ['accepted_layouts', 'declined_layouts']}))
if report['failures']:
    raise SystemExit(1)
