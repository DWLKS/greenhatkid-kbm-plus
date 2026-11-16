"""Builds the MouseCamera-Native game patch (IPS32) for the Green Hat Kid game, main build E0F3CCFAF50065AE...

    python build_native_camera_ips.py [--llvm <dir with clang/ld.lld/llvm-objcopy>] [--main <main.elf>] [--out <dir>]

What it does
  1. Compiles cave.c + hooks.S for AArch64 and links them at a fixed address in the unused space after the
     game's code (module offset 0x3C46400; the Dynamic FPS patch uses 0x3C46200-0x3C46277).
  2. Replaces one game instruction with a branch into that code: module 0x0625370 `str s0, [sp, #12]`
     in the player camera update, where the right stick has just been turned into this frame's yaw and pitch.
  3. Writes <out>/MouseCamera-Native/exefs/<build id>.ips (IPS32: offsets are module offset + 0x100).

--main is optional: when given (a decompressed main as an ELF at 0x80b75000, e.g. from nso2elf.py), the
script checks that the replaced instruction and the cave area are what it expects before writing.
"""
import argparse
import os
import struct
import subprocess
import sys
import tempfile

BUILD_ID = 'E0F3CCFAF50065AE4EA01EB5AEB13B1A45DA3348'
ELF_BASE = 0x80B75000                 # address the offsets below are written against
CAVE = 0x3C46400                      # module offset of the code cave
CAVE_END = 0x3C46C00                  # another mod (ultrawide) puts its code here
STATE = 0x4E9FE40                     # module offset of spare BSS for the state (Dynamic FPS uses 0x4E9FE00-0x4E9FE0F)
GET_MOUSE_STATES = 0x3C44800          # module offset of the game's PLT stub for nn::hid::GetMouseStates
HOOKS = [  # module offset, expected original instruction, symbol, kind ('bl' returns with ret; 'b' jumps back)
    (0x0625370, 0xBD000FE0, 'ghk_hook_rates', 'bl'),      # str s0, [sp, #12]   (player camera: stick -> turn this frame)
    (0x27A05F4, 0x1F000500, 'ghk_hook_aim_gyro_pitch', 'bl'),  # fmadd s0, s8, s0, s1   (aim camera, motion path: pitch)
    (0x27A0628, 0x1E202820, 'ghk_hook_aim_pitch', 'bl'),       # fadd s0, s1, s0        (aim camera, stick path: pitch)
    (0x27A07BC, 0x1F0A0400, 'ghk_hook_aim_gyro_yaw', 'bl'),    # fmadd s0, s0, s10, s1  (aim camera, motion path: yaw)
    (0x27A0828, 0x1E202820, 'ghk_hook_aim_yaw', 'bl'),         # fadd s0, s1, s0        (aim camera, stick path: yaw)
    (0x27B7314, 0x2A0003F8, 'ghk_hook_horse_mode', 'bl'),      # mov w24, w0            (horse camera: stick idle bit)
    (0x27B7E28, 0xBD40AE61, 'ghk_hook_horse_yaw', 'bl'),       # ldr s1, [x19, #0xac]   (horse camera, by hand: yaw)
    (0x27B7F78, 0xBD40A661, 'ghk_hook_horse_pitch', 'bl'),     # ldr s1, [x19, #0xa4]   (horse camera, by hand: pitch)
]
BACKS = {}
AIM = STATE + 0x40                    # aiming state: yaw left for the yaw hook, call counters
HORSE = STATE + 0x50                  # riding state: yaw/pitch left for the turn hooks, call counters
NSO_HEADER = 0x100
HERE = os.path.dirname(os.path.abspath(__file__))


def run(cmd):
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit('command failed: ' + ' '.join(cmd) + '\n' + r.stdout + r.stderr)
    return r.stdout


def bl(src, dst):
    off = (dst - src) // 4
    if not -(1 << 25) <= off < (1 << 25):
        sys.exit(f'branch out of range: {src:#x} -> {dst:#x}')
    return 0x94000000 | (off & 0x3FFFFFF)


def branch(src, dst):
    off = (dst - src) // 4
    if not -(1 << 25) <= off < (1 << 25):
        sys.exit(f'branch out of range: {src:#x} -> {dst:#x}')
    return 0x14000000 | (off & 0x3FFFFFF)


def symbols(nm, elf):
    out = {}
    for line in run([nm, elf]).splitlines():
        p = line.split()
        if len(p) == 3:
            out[p[2]] = int(p[0], 16)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--llvm', default='', help='folder of clang, ld.lld, llvm-objcopy and llvm-nm (default: from PATH)')
    ap.add_argument('--main', default='')
    ap.add_argument('--out', default=os.path.join(HERE, '..'))
    a = ap.parse_args()
    clang, lld = os.path.join(a.llvm, 'clang'), os.path.join(a.llvm, 'ld.lld')
    objcopy, nm = os.path.join(a.llvm, 'llvm-objcopy'), os.path.join(a.llvm, 'llvm-nm')

    with tempfile.TemporaryDirectory() as t:
        flags = ['--target=aarch64-none-elf', '-mcpu=cortex-a78', '-O2', '-ffreestanding', '-fno-builtin',
                 '-fno-stack-protector', '-fno-pic', '-fno-asynchronous-unwind-tables', '-mno-outline-atomics',
                 '-fno-common', '-ffixed-x18', '-Wall', '-Werror']
        run([clang] + flags + ['-c', os.path.join(HERE, 'cave.c'), '-o', os.path.join(t, 'cave.o')])
        run([clang, '--target=aarch64-none-elf', '-c', os.path.join(HERE, 'hooks.S'), '-o', os.path.join(t, 'hooks.o')])
        ld = os.path.join(t, 'link.ld')
        with open(ld, 'w') as f:
            f.write(f'''ENTRY(ghk_hook_rates)
ghk_mcam_state = {ELF_BASE + STATE:#x};
ghk_mcam_aim = {ELF_BASE + AIM:#x};
ghk_mcam_horse = {ELF_BASE + HORSE:#x};
{chr(10).join(f'{n} = {ELF_BASE + o:#x};' for n, o in BACKS.items())}
_ZN2nn3hid14GetMouseStatesEPNS0_10MouseStateEi = {ELF_BASE + GET_MOUSE_STATES:#x};
SECTIONS {{
  . = {ELF_BASE + CAVE:#x};
  .cave : {{ *(.text .text.*) *(.rodata .rodata.*) }}
  /DISCARD/ : {{ *(.comment) *(.note*) *(.eh_frame*) }}
}}
''')
        elf = os.path.join(t, 'cave.elf')
        run([lld, '-T', ld, '--no-pie', '-static', '-o', elf, os.path.join(t, 'hooks.o'), os.path.join(t, 'cave.o')])
        sects = run([os.path.join(a.llvm, 'llvm-readobj'), '--elf-output-style=GNU', '-S', elf])
        for bad in ('.data', '.bss'):
            if f' {bad} ' in sects or f' {bad}.' in sects:
                sys.exit(f'the cave must not have {bad}: everything has to live in the code segment')
        blob_path = os.path.join(t, 'cave.bin')
        run([objcopy, '-O', 'binary', '--only-section=.cave', elf, blob_path])
        blob = open(blob_path, 'rb').read()
        sym = symbols(nm, elf)

    if CAVE + len(blob) > CAVE_END:
        sys.exit(f'cave too large: {len(blob)} bytes, room for {CAVE_END - CAVE}')
    if b'GHKMCAM1' not in blob:
        sys.exit('settings block missing from the cave')

    patches = [(CAVE, blob)]
    for off, expect, name, kind in HOOKS:
        enc = bl if kind == 'bl' else branch
        patches.append((off, struct.pack('<I', enc(ELF_BASE + off, sym[name]))))

    if a.main:
        img = open(a.main, 'rb').read()
        def at(off, n):  # raw module dump (module offset 0 = file offset 0), taken with the old patch applied
            return img[off:off + n]
        for off, expect, name, kind in HOOKS:
            if off == 0x0625370:
                continue  # holds the previous version's branch in a dump of the patched game
            got = struct.unpack('<I', at(off, 4))[0]
            if got != expect:
                sys.exit(f'{name}: game instruction at {off:#x} is {got:#010x}, expected {expect:#010x} (other game version?)')
        print('main checked: hook instructions and cave area match')

    out_dir = os.path.join(a.out, 'MouseCamera-Native', 'exefs')
    os.makedirs(out_dir, exist_ok=True)
    ips = bytearray(b'IPS32')
    for off, data in patches:
        ips += struct.pack('>IH', off + NSO_HEADER, len(data)) + data
    ips += b'EEOF'
    path = os.path.join(out_dir, BUILD_ID + '.ips')
    open(path, 'wb').write(ips)
    print(f'cave {len(blob)} bytes at module {CAVE:#x}; hooks: ' +
          ', '.join(f'{n} {sym[n]:#x}' for _, _, n, _ in HOOKS))
    print('wrote', os.path.normpath(path))


if __name__ == '__main__':
    main()
