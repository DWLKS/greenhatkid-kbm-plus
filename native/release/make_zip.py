"""Builds the release zip of the native version: out/GreenHatKid-KBM-Plus-<version>.zip
  GreenHatKid-KBM-Plus-<version>/GreenHatKid-KBM-Plus.exe, README.txt, LICENSES.txt
usage: python native/release/make_zip.py   (after `cargo build --release` and tools/make_notices.py)"""
import hashlib, os, re, zipfile
NATIVE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REPO = os.path.dirname(NATIVE)
version = re.search(r'^version = "([^"]+)"', open(os.path.join(NATIVE, 'Cargo.toml')).read(), re.M).group(1)
top = 'GreenHatKid-KBM-Plus-%s' % version
files = [('GreenHatKid-KBM-Plus.exe', os.path.join(NATIVE, 'target', 'release', 'GreenHatKid-KBM-Plus.exe')),
         ('README.txt', os.path.join(NATIVE, 'release', 'README.txt')),
         ('LICENSES.txt', os.path.join(NATIVE, 'LICENSES.txt'))]
os.makedirs(os.path.join(REPO, 'out'), exist_ok=True)
dst = os.path.join(REPO, 'out', top + '.zip')
with zipfile.ZipFile(dst, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    for name, src in files:
        data = open(src, 'rb').read()
        if name.endswith('.txt'):
            data = data.replace(b'\r\n', b'\n').replace(b'\n', b'\r\n')     # Notepad-friendly
        z.writestr(top + '/' + name, data)
exe = open(files[0][1], 'rb').read()
print(dst, os.path.getsize(dst), 'bytes; exe sha256', hashlib.sha256(exe).hexdigest())
