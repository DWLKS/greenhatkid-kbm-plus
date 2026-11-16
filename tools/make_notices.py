"""Writes native/LICENSES.txt, the one license file of the release: 1. this app's license (LICENSE),
2. the original mod's notice (third_party/original/LICENSE, word for word), 3. every crate built into the
Windows exe with its license files (identical texts printed once).
usage: python tools/make_notices.py   (needs cargo on PATH)"""
import hashlib, json, os, subprocess
REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
NATIVE = os.path.join(REPO, 'native')
run = lambda *a: subprocess.run(a, cwd=NATIVE, capture_output=True, text=True, check=True).stdout
used = {l.split()[0] + ' ' + l.split()[1].lstrip('v') for l in run('cargo', 'tree', '--prefix', 'none', '-e', 'normal',
        '--target', 'x86_64-pc-windows-msvc', '--format', '{p}').splitlines() if l.strip()}
meta = json.loads(run('cargo', 'metadata', '--format-version', '1', '--filter-platform', 'x86_64-pc-windows-msvc'))
pkgs = sorted((p for p in meta['packages'] if p['name'] + ' ' + p['version'] in used and p['source']), key=lambda p: p['name'])
texts, order, rows = {}, [], []
for p in pkgs:
    d = os.path.dirname(p['manifest_path'])
    files = sorted(f for f in os.listdir(d) if f.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'NOTICE')) and os.path.isfile(os.path.join(d, f)))
    ids = []
    for f in files:
        t = open(os.path.join(d, f), encoding='utf-8', errors='replace').read().strip()
        h = hashlib.sha1(t.encode()).hexdigest()[:8]
        if h not in texts:
            texts[h] = (t, '%s %s/%s' % (p['name'], p['version'], f)); order.append(h)
        ids.append(h)
    rows.append('%-28s %-10s %-45s texts: %s' % (p['name'], p['version'], p.get('license') or '?', ', '.join(ids) or 'no license file; authors: ' + ', '.join(a.split(' <')[0] for a in p.get('authors') or ['?'])))
rule = lambda title: ['', '=' * 100, title, '=' * 100, '']
read = lambda p: open(os.path.join(REPO, p), encoding='utf-8').read().strip()
out = ['Green Hat Kid Keyboard & Mouse Plus - licenses', '',
       '1. This app', '2. The original mod it is based on (lichii-keyboard-mouse)',
       '3. Open-source components compiled into the exe']
out += rule('1. This app') + [read('LICENSE')]
out += rule('2. The original mod: lichii-keyboard-mouse (its license notice, unchanged)') + [read('third_party/original/LICENSE')]
out += rule('3. Open-source components compiled into the exe') + [
       'ARM astcenc (Apache-2.0) is included through ctt-astcenc; zstd (BSD) through zstd-sys.', '']
out += rows + ['', 'Crates without a license file are offered under the licenses named above; the MIT and Apache-2.0',
                'texts below apply to them, with their authors as the copyright holders.', '', '=' * 100]
for h in order:
    t, src = texts[h]
    out += ['', '[%s] from %s' % (h, src), '-' * 100, t]
open(os.path.join(NATIVE, 'LICENSES.txt'), 'w', encoding='utf-8', newline='\r\n').write('\n'.join(out) + '\n')
print(len(rows), 'crates,', len(order), 'license texts')
