"""Build runtime packages from release binaries. Run from the repository root."""
import argparse
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import tarfile
import tomllib
import zipfile

p = argparse.ArgumentParser()
p.add_argument('--target', required=True)
a = p.parse_args()
version = tomllib.loads(Path('app/Cargo.toml').read_text())['package']['version']
if os.environ.get('GITHUB_OUTPUT'):
    with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8') as output:
        output.write(f'version={version}\n')
source = Path('target') / a.target / 'release'
dist = Path('dist')
assets = Path('release-assets')
shutil.rmtree(dist, ignore_errors=True)
dist.mkdir()
assets.mkdir(exist_ok=True)
windows = 'windows' in a.target
mac = 'apple' in a.target
suffix = '.exe' if windows else ''
names = [('tf2-demo-toolkit', 'TF2_Demo_Toolkit'), ('tf2-demo-director', 'TF2_Demo_Director'), ('export_all', 'export_all')]
root = dist
if mac:
    root = dist / 'TF2 Demo Toolkit.app' / 'Contents' / 'MacOS'
    root.mkdir(parents=True)
for original, packaged in names:
    shutil.copy2(source / (original + suffix), root / (packaged + suffix))
data_root = root.parent / 'Resources' if mac else root
data_root.mkdir(exist_ok=True)
shutil.copytree('recording_resources_archive', data_root / 'recording_resources_archive')
for file in ['README.md', 'THIRD_PARTY_NOTICES.md']:
    shutil.copy2(file, data_root / file)
(data_root / 'BUILD.txt').write_text(f"Commit: {os.environ.get('GITHUB_SHA', 'local')}\nTarget: {a.target}\n", encoding='utf-8')
(data_root / 'START-HERE.txt').write_text('TF2 Demo Toolkit beta\n\nNo Rust, Cargo, Visual Studio, Python or .NET installation is needed.\nKeep all files in this package together.\nWindows: run TF2_Demo_Toolkit.exe (or use the Setup.exe download).\nmacOS: open TF2 Demo Toolkit.app.\nLinux: run ./TF2_Demo_Toolkit.sh from a graphical desktop.\n\nLinux requires glibc 2.35+ and a working X11/Wayland desktop and graphics drivers.\nTF2, HLAE and FFmpeg are separate recording requirements.\nHLAE recording is Windows-only.\nBeta builds are not publisher-signed or Apple-notarized.\n', encoding='utf-8')

if mac:
    contents = root.parent
    with (contents / 'Info.plist').open('wb') as f:
        plistlib.dump({'CFBundleExecutable': 'TF2_Demo_Toolkit', 'CFBundleIdentifier': 'com.dc1818.tf2-demo-toolkit', 'CFBundleName': 'TF2 Demo Toolkit', 'CFBundlePackageType': 'APPL', 'CFBundleShortVersionString': version, 'CFBundleVersion': os.environ.get('GITHUB_RUN_NUMBER', '1'), 'NSHighResolutionCapable': True, 'LSMinimumSystemVersion': '11.0'}, f)
    for _, name in names:
        output = subprocess.check_output(['otool', '-L', str(root / name)], text=True)
        for line in output.splitlines()[1:]:
            dependency = line.strip().split(' (')[0]
            if not dependency.startswith(('/System/Library/', '/usr/lib/')):
                raise RuntimeError(f'Unbundled macOS dependency: {dependency}')
        if name != 'TF2_Demo_Toolkit':
            subprocess.run(['codesign', '--force', '--sign', '-', str(root / name)], check=True)
    subprocess.run(['codesign', '--force', '--sign', '-', str(contents.parent)], check=True)
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(contents.parent)], check=True)
    arch = 'arm64' if a.target.startswith('aarch64') else 'x64'
    subprocess.run(['ditto', '-c', '-k', '--sequesterRsrc', '--keepParent', str(contents.parent), str(assets / f'TF2-Demo-Toolkit-macOS-{arch}.zip')], check=True)
elif windows:
    with zipfile.ZipFile(assets / 'TF2-Demo-Toolkit-Windows-x64-Portable.zip', 'w', zipfile.ZIP_DEFLATED) as z:
        for file in dist.rglob('*'):
            if file.is_file():
                z.write(file, file.relative_to(dist))
else:
    lib = root / 'lib'
    lib.mkdir()
    # glibc and the ELF loader must come from the host as a matched pair.
    system = re.compile(r'^(ld-linux|lib(c|m|pthread|dl|rt|resolv|util|anl)\.so)')
    queue = [root / name for _, name in names]
    # winit loads these at runtime, so they do not necessarily appear in ldd.
    cache = subprocess.check_output(['ldconfig', '-p'], text=True)
    for name in ['libX11.so.6', 'libX11-xcb.so.1', 'libXcursor.so.1', 'libXi.so.6', 'libXrandr.so.2', 'libxkbcommon.so.0', 'libxkbcommon-x11.so.0', 'libwayland-client.so.0', 'libwayland-cursor.so.0', 'libwayland-egl.so.1', 'libfontconfig.so.1']:
        match = re.search(r'^\s*' + re.escape(name) + r' .*=> (.+)$', cache, re.M)
        if not match:
            raise RuntimeError(f'Missing runtime library {name}')
        dest = lib / name
        shutil.copy2(match[1], dest)
        queue.append(dest)
    seen = set()
    while queue:
        binary = queue.pop()
        if binary in seen:
            continue
        seen.add(binary)
        output = subprocess.check_output(['ldd', str(binary)], text=True)
        if 'not found' in output:
            raise RuntimeError(output)
        for name, path in re.findall(r'(\S+) => (/\S+)', output):
            if system.match(name) or (lib / name).exists():
                continue
            dest = lib / name
            shutil.copy2(path, dest)
            queue.append(dest)
    # Preserve distribution copyright notices for the bundled shared libraries.
    notices = root / 'library-notices'
    notices.mkdir()
    for file in lib.iterdir():
        matches = subprocess.run(['dpkg-query', '-S', '*/' + file.name], capture_output=True, text=True)
        for line in matches.stdout.splitlines():
            package = line.split(': ')[0].split(':')[0]
            copyright_file = Path('/usr/share/doc') / package / 'copyright'
            if copyright_file.is_file():
                shutil.copy2(copyright_file, notices / (package + '.copyright'))
    for _, name in names[:2]:
        launcher = root / (name + '.sh')
        launcher.write_text('#!/bin/sh\nHERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\nexport LD_LIBRARY_PATH="$HERE/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"\nexport SLINT_BACKEND="${SLINT_BACKEND:-winit-software}"\nexec "$HERE/' + name + '" "$@"\n')
        launcher.chmod(0o755)
    with tarfile.open(assets / 'TF2-Demo-Toolkit-Linux-x64.tar.gz', 'w:gz') as t:
        t.add(dist, arcname='TF2-Demo-Toolkit')

print(f'Packaged {a.target}')
