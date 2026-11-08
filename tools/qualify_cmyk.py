#!/usr/bin/env python3
"""Bounded multi-CMS witness matrix; keep scalars/hashes, not output planes.

Every profile path is read explicitly; missing files/reader failures are reported.
Input uses normalized ICC ink. Argyll/raw Mox are compared with BPC-off LCMS;
production adapters are compared with relative+BPC LCMS separately.
"""
import argparse
import array
import hashlib
import itertools
import json
import math
from pathlib import Path
import struct
import subprocess


def sha(data):
    return hashlib.sha256(data).hexdigest()


def samples(quick):
    rows = list(itertools.product([0.0, 0.5, 1.0], repeat=4))
    levels = [i / 256 for i in range(257)]
    for x in levels:
        rows.extend([(x, x, x, 0.0), (0.0, 0.0, 0.0, x)])
        for c in range(4):
            p = [0.0] * 4
            p[c] = x
            rows.append(tuple(p))
    state = 0x116C2026
    for _ in range(64 if quick else 4096):
        p = []
        for _ in range(4):
            state = (1664525 * state + 1013904223) & 0xffffffff
            p.append((state >> 8) / 16777215)
        rows.append(tuple(p))
    rows.extend([(0.7137, 0.0, 0.3294, 0.0), (0.7653, 0.0399, 0.4082, 0.0187)])
    return [tuple(struct.unpack('=4f', struct.pack('=4f', *p))) for p in rows]


def stats(a, b, rows):
    if len(a) != len(b) or len(a) != len(rows) * 3:
        raise ValueError('oracle output extent mismatch')
    if not all(math.isfinite(x) for x in a + b):
        raise ValueError('nonfinite oracle output')
    delta = [abs(x-y) for x, y in zip(a, b)]
    i = max(range(len(delta)), key=delta.__getitem__)
    clipped = [abs(min(1.0, max(0.0, x))-min(1.0, max(0.0, y))) for x, y in zip(a, b)]
    interior = [d for d, ref in zip(delta, b) if 1/255 < ref < 254/255]
    q8 = lambda x: int(min(1.0, max(0.0, x))*255+0.5)
    q16 = lambda x: int(min(1.0, max(0.0, x))*65535+0.5)
    return {'max_abs_rgb': delta[i], 'mean_abs_rgb': sum(delta)/len(delta),
            'clipped_max_abs_rgb': max(clipped), 'clipped_over_0_0013': sum(x > 0.0013 for x in clipped),
            'interior_reference_max': max(interior, default=0), 'interior_components': len(interior),
            'max_quantized_u8_difference': max(abs(q8(x)-q8(y)) for x, y in zip(a,b)),
            'max_quantized_u16_difference': max(abs(q16(x)-q16(y)) for x, y in zip(a,b)),
            'actual_outside_unit': sum(x < 0 or x > 1 for x in a),
            'reference_outside_unit': sum(x < 0 or x > 1 for x in b),
            'over_0_0013': sum(x > 0.0013 for x in delta), 'components': len(delta),
            'worst': {'sample': i//3, 'ink': rows[i//3], 'channel': i%3,
                      'actual': a[i], 'reference': b[i]},
            'actual_range': [min(a), max(a)]}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ['profiles-root', 'c-lcms', 'rust-eval', 'argyll', 'output']:
        p.add_argument('--'+name, required=True, type=Path)
    p.add_argument('--quick', action='store_true')
    args = p.parse_args()
    args.output.mkdir(exist_ok=False, parents=True)
    root = args.profiles_root.resolve()
    rows = samples(args.quick)
    inputs = args.output/'inputs.f32'
    inputs.write_bytes(b''.join(struct.pack('=4f', *x) for x in rows))
    profiles = {}
    for path in sorted(root.rglob('*')):
        if path.suffix.lower() not in ('.icc', '.icm'):
            continue
        data = path.read_bytes()
        if len(data) >= 128 and data[36:40] == b'acsp' and data[16:20] == b'CMYK':
            if data[12:16] != b'link':
                profiles.setdefault(sha(data), path)
    if not profiles:
        raise ValueError('no CMYK profiles')
    targets = [root/'crates/argyll-sys/sRGB.icm',
               root/'external/argyllcms/icc/DisplayP3.icm']
    report = {'scope': 'F32 CMYK to RGB witnesses; independent CMS differences are measurements, not new tolerances',
              'policies': {'argyll_and_raw_mox': 'literal intent, no added BPC; compare matching C intent without BPC',
                           'production_lcms': 'relative+BPC',
                           'production_mox': 'existing target-dependent intent/BPC policy; compare C policies separately'},
              'quick': args.quick, 'samples': len(rows), 'input_sha256': sha(inputs.read_bytes()),
              'engines': {n: {'path': str(v.resolve()), 'sha256': sha(v.read_bytes())}
                          for n, v in [('c_lcms', args.c_lcms), ('rust', args.rust_eval), ('argyll', args.argyll)]},
              'profiles': [{'path': str(v.relative_to(root)), 'sha256': k, 'bytes': v.stat().st_size}
                           for k, v in profiles.items()], 'cells': []}
    for digest, source in profiles.items():
        for target in targets:
            # All modes use the exact same source and destination bytes.
            values = {}
            for label, exe, mode in [('c0', args.c_lcms, '0'), ('c1', args.c_lcms, '1'),
                 ('c2', args.c_lcms, '2'), ('c3', args.c_lcms, '3'), ('c1bpc', args.c_lcms, '1:bpc'),
                 ('rust0', args.rust_eval, '0'), ('rust1', args.rust_eval, '1'),
                 ('rust2', args.rust_eval, '2'), ('rust3', args.rust_eval, '3'),
                 ('rust1bpc', args.rust_eval, '1:bpc'), ('production-lcms', args.rust_eval, 'production'),
                 ('production-mox', args.rust_eval, 'mox'), ('raw-mox', args.rust_eval, 'mox-raw:1'),
                 ('argyll', args.argyll, '1'), ('argyll0', args.argyll, '0'),
                 ('argyll2', args.argyll, '2'), ('argyll3', args.argyll, '3'),
                 ('raw-mox0', args.rust_eval, 'mox-raw:0'), ('raw-mox2', args.rust_eval, 'mox-raw:2'),
                 ('raw-mox3', args.rust_eval, 'mox-raw:3')]:
                result = subprocess.run([str(exe.resolve()), str(source), str(target), str(inputs), '-', mode],
                                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
                cell = {'profile': str(source.relative_to(root)), 'profile_sha256': digest,
                        'target': target.name, 'target_sha256': sha(target.read_bytes()),
                        'engine': label, 'intent_mode': mode, 'exit': result.returncode,
                        'stderr': result.stderr.decode(errors='replace')[-2048:]}
                if result.returncode == 0:
                    floats = list(array.array('f', result.stdout))
                    if len(floats) != len(rows) * 3:
                        raise ValueError('wrong output extent: '+str(cell))
                    if not all(math.isfinite(x) for x in floats):
                        raise ValueError('nonfinite output: '+str(cell))
                    cell['output_sha256'] = sha(result.stdout)
                    values[label] = floats
                    reference = {'rust0': 'c0', 'rust1': 'c1', 'rust2': 'c2', 'rust3': 'c3',
                        'rust1bpc': 'c1bpc', 'production-lcms': 'c1bpc', 'production-mox': 'c1bpc',
                        'raw-mox': 'c1', 'argyll': 'c1', 'argyll0': 'c0', 'argyll2': 'c2', 'argyll3': 'c3',
                        'raw-mox0': 'c0', 'raw-mox2': 'c2', 'raw-mox3': 'c3'}.get(label)
                    if reference in values:
                        cell['comparison'] = stats(floats, values[reference], rows)
                        cell['reference'] = reference
                    if label == 'production-mox':
                        cell['intent_controls'] = {ref: stats(floats, values[ref], rows)
                                                   for ref in ['c0', 'c1', 'c1bpc'] if ref in values}
                report['cells'].append(cell)
                # Save every terminal result immediately, including failures.
                (args.output/'report.json').write_text(json.dumps(report, indent=2)+'\n')
            print(json.dumps({'profile': str(source.relative_to(root)), 'target': target.name,
                              'accepted': sorted(values)}), flush=True)
    summary = {}
    for cell in report['cells']:
        s = summary.setdefault(cell['engine'], {'accepted': 0, 'failed': 0, 'max_abs_rgb': 0.0,
                                                'over_0_0013': 0, 'components': 0, 'clipped_max_abs_rgb': 0.0, 'clipped_over_0_0013': 0})
        s['accepted' if cell['exit'] == 0 else 'failed'] += 1
        if 'comparison' in cell:
            c = cell['comparison']
            if c['max_abs_rgb'] > s['max_abs_rgb']:
                s['max_abs_rgb'] = c['max_abs_rgb']; s['worst_profile'] = cell['profile'];s['worst_target'] = cell['target']; s['worst'] = c['worst']
            s['over_0_0013'] += c['over_0_0013'];s['components'] += c['components']
            s['clipped_max_abs_rgb'] = max(s['clipped_max_abs_rgb'], c['clipped_max_abs_rgb'])
            s['clipped_over_0_0013'] += c['clipped_over_0_0013']
    report['summary'] = summary
    (args.output/'report.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(summary, indent=2))
    if any(c['exit'] for c in report['cells']):
        raise SystemExit('CMS execution failures; see report.json')
    if any(c.get('comparison', {}).get('max_abs_rgb', 0) != 0
           for c in report['cells'] if c['engine'].startswith('rust') or c['engine'] == 'production-lcms'):
        raise SystemExit('C/Rust LCMS mismatch; see report.json')


if __name__ == '__main__':
    main()
