#!/usr/bin/env python3
"""Compare the SAME native JXL input with common ICCs and BPC disabled.

This is a policy-isolation control, distinct from production relative+BPC.
Only scalar comparisons/hashes are saved; RGB outputs remain in memory.
"""
import argparse
import array
import json
from pathlib import Path
import struct
import subprocess
from qualify_cmyk import sha, stats

p = argparse.ArgumentParser(description=__doc__)
for name in ['source', 'rgb-profile', 'inverted-input', 'c-lcms', 'rust-eval', 'argyll', 'output']:
    p.add_argument('--'+name, required=True, type=Path)
a = p.parse_args()
a.output.mkdir(parents=True, exist_ok=False)
original = a.inverted_input.read_bytes()
assert len(original) % 16 == 0
native = array.array('f', original)
normalized = [struct.unpack('=f',struct.pack('=f',1.0-v))[0] for v in native]
rows = [tuple(normalized[i:i+4]) for i in range(0,len(normalized),4)]
inputs = a.output/'normalized-ink.f32'
inputs.write_bytes(array.array('f', normalized).tobytes())
report = {'scope': 'same native image; exact common ICCs, relative intent, BPC disabled',
          'original_input_sha256': sha(original), 'normalized_input_sha256': sha(inputs.read_bytes()),
          'source_sha256': sha(a.source.read_bytes()), 'target_sha256': sha(a.rgb_profile.read_bytes()),
          'pixels': len(rows), 'cells': {}}
values = {}
for name, exe, mode in [('c-lcms',a.c_lcms,'1'),('rust-lcms',a.rust_eval,'1'),
                        ('mox-raw',a.rust_eval,'mox-raw:1'),('argyll',a.argyll,'1')]:
    result = subprocess.run([str(exe.resolve()),str(a.source),str(a.rgb_profile),str(inputs),'-',mode],capture_output=True)
    cell = {'exit':result.returncode,'stderr':result.stderr.decode(errors='replace')}
    if result.returncode == 0:
        values[name] = list(array.array('f', result.stdout))
        cell['output_sha256'] = sha(result.stdout)
        if name != 'c-lcms': cell['comparison'] = stats(values[name],values['c-lcms'],rows)
    report['cells'][name]=cell
    (a.output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
if any(x['exit'] for x in report['cells'].values()):raise SystemExit(1)
