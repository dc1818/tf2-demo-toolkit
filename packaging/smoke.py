"""Check packaged dependencies and startup without development tools on PATH."""
import os
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import plistlib

root = Path('dist').resolve()
windows = sys.platform == 'win32'
mac = sys.platform == 'darwin'
if mac:
    root = root / 'TF2 Demo Toolkit.app' / 'Contents' / 'MacOS'
suffix = '.exe' if windows else ''
if windows:
    import pefile
    for file in root.glob('*.exe'):
        pe = pefile.PE(str(file))
        imports = [entry.dll.decode().lower() for entry in getattr(pe, 'DIRECTORY_ENTRY_IMPORT', [])]
        imports += [entry.dll.decode().lower() for entry in getattr(pe, 'DIRECTORY_ENTRY_DELAY_IMPORT', [])]
        print(file.name, imports)
        forbidden = [name for name in imports if name.startswith(('vcruntime', 'msvcp', 'msvcr', 'concrt'))]
        if forbidden:
            raise RuntimeError(f'External MSVC runtime required: {forbidden}')
        if file.name == 'TF2_Demo_Toolkit.exe':
            resource_types = {entry.id for entry in pe.DIRECTORY_ENTRY_RESOURCE.entries}
            if not {3, 14}.issubset(resource_types):
                raise RuntimeError('Application executable is missing its icon resources')
if mac:
    contents = root.parent
    with (contents / 'Info.plist').open('rb') as file:
        info = plistlib.load(file)
    icon = contents / 'Resources' / info['CFBundleIconFile']
    if icon.read_bytes()[:4] != b'icns':
        raise RuntimeError('Application bundle is missing a valid ICNS icon')
if not windows and not mac:
    with tempfile.TemporaryDirectory(prefix='toolkit-desktop-') as directory:
        launcher_env = os.environ.copy()
        launcher_env['XDG_DATA_HOME'] = directory
        subprocess.run(['sh', str(root / 'install-desktop.sh')], env=launcher_env, check=True)
        app_id = 'com.dc1818.tf2-demo-toolkit'
        desktop = Path(directory) / 'applications' / (app_id + '.desktop')
        if f'Icon={app_id}' not in desktop.read_text():
            raise RuntimeError('Linux desktop entry does not reference the app icon')
        for size in (16, 24, 32, 48, 64, 128, 256, 512):
            icon = Path(directory) / f'icons/hicolor/{size}x{size}/apps/{app_id}.png'
            if not icon.is_file():
                raise RuntimeError(f'Linux launcher is missing its {size}px icon')
env = os.environ.copy()
env['PATH'] = str(Path(os.environ['SystemRoot']) / 'System32') if windows else '/usr/bin:/bin'
env['SLINT_BACKEND'] = 'winit-software'
if not windows and not mac:
    env['LD_LIBRARY_PATH'] = str(root / 'lib')
# The parser must start successfully enough to report its own usage error.
with tempfile.TemporaryDirectory() as working:
    result = subprocess.run([str(root / ('export_all' + suffix))], env=env, cwd=working, capture_output=True, text=True, timeout=30)
    output = result.stdout + result.stderr
    print(output)
    if 'usage' not in output.lower():
        raise RuntimeError('Packaged parser did not reach its argument handling')
    director = subprocess.run([str(root / ('TF2_Demo_Toolkit' + suffix)), '--director'], env=env, cwd=working, capture_output=True, text=True, timeout=30)
    if director.returncode != 1 or 'usage' not in (director.stdout + director.stderr).lower():
        raise RuntimeError('Packaged Director did not reach session argument handling')
    # Exercise the integrated UI as well as CLI dispatch, with a session path
    # containing spaces and no game or development tools available.
    session = Path(working) / 'Director session.json'
    session.write_text(json.dumps({
        'schema_version': 7, 'candidate_id': 'smoke', 'demo_file': 'smoke.dem',
        'map_name': 'cp_process', 'start_tick': 100, 'end_tick': 200,
        'cues': [], 'whole_candidate_tags': [], 'shortcuts': [],
        'campath_file': str(Path(working) / 'campath.xml'),
        'output_directory': working, 'telemetry_log': str(Path(working) / 'console.log'),
        'telemetry_marker_prefix': 'TF2FRAG_DIRECTOR_TICK',
        'command_cfg_directory': working, 'control': {'mode': 'hotkeys_only'},
    }), encoding='utf-8')
    (Path(working) / 'console.log').write_text('TF2FRAG_MANUAL_PAUSED_AT_START\n', encoding='utf-8')
    for arguments in [[], ['--director', str(session)]]:
        name = 'TF2_Demo_Toolkit'
        with tempfile.TemporaryFile() as log:
            process = subprocess.Popen([str(root / (name + suffix)), *arguments], env=env, cwd=working, stdout=log, stderr=log)
            try:
                time.sleep(12)
                if process.poll() is not None:
                    log.seek(0)
                    raise RuntimeError(f'{name} exited during startup ({process.returncode}): {log.read().decode(errors="replace")}')
                if arguments:
                    diagnostic = (Path(working) / 'director_telemetry.log').read_text(encoding='utf-8')
                    if 'status=DIRECTOR_WINDOWS_READY' not in diagnostic:
                        raise RuntimeError('Director did not open its overlay windows')
                print(f'{name} {arguments}: startup OK with restricted PATH')
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=20)
