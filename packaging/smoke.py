"""Check packaged dependencies and startup without development tools on PATH."""
import os
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time

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
    for arguments in [[], ['--director', str(session)]]:
        name = 'TF2_Demo_Toolkit'
        with tempfile.TemporaryFile() as log:
            process = subprocess.Popen([str(root / (name + suffix)), *arguments], env=env, cwd=working, stdout=log, stderr=log)
            try:
                time.sleep(12)
                if process.poll() is not None:
                    log.seek(0)
                    raise RuntimeError(f'{name} exited during startup ({process.returncode}): {log.read().decode(errors="replace")}')
                print(f'{name} {arguments}: startup OK with restricted PATH')
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=20)
