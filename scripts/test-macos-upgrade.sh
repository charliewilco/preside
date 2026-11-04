#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $# -lt 1 || $# -gt 2 ]]; then
  printf 'Usage: %s <prior-commit> [current-app-path]\n' "$0" >&2
  exit 2
fi
exec /usr/bin/python3 - "$@" <<'PY'
import hashlib
import json
import os
import pathlib
import shlex
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request

root = pathlib.Path.cwd()
prior_ref = subprocess.check_output(
    ['git', 'rev-parse', '--verify', sys.argv[1] + '^{commit}'], text=True
).strip()
current_ref = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
current_app = pathlib.Path(sys.argv[2] if len(sys.argv) == 3 else 'dist/Preside.app').resolve()
if not (current_app / 'Contents/Resources/pp').is_file():
    sys.exit('Build the current app with scripts/build-macos.sh before running this test.')
workspace = pathlib.Path(tempfile.mkdtemp(prefix='preside-upgrade-smoke-'))
home = workspace / 'isolated home'
home.mkdir()
installed = workspace / 'Applications with spaces/Preside.app'
previous = workspace / 'Previous Preside.app'
old_source = workspace / 'source'
old_source.mkdir()
environment = {
    'HOME': str(home),
    'XDG_CONFIG_HOME': str(home / '.config'),
    'XDG_STATE_HOME': str(home / '.local/state'),
    'PATH': '/usr/bin:/bin:/usr/sbin:/sbin',
}
created = []
owned = {}
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))


def cli_path(app):
    return app / 'Contents/Resources/pp'


def cli(arguments, app=None, check=True):
    return subprocess.run(
        [str(cli_path(app or installed))] + arguments,
        env=environment, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        timeout=30, check=check,
    )


def status(name, app=None):
    return json.loads(cli(['status', '--json', '--', name], app).stdout)


def available_port():
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        return listener.getsockname()[1]


def fetch(port, expected):
    deadline = time.monotonic() + 5
    last_error = None
    while time.monotonic() < deadline:
        try:
            with opener.open('http://127.0.0.1:%d/' % port, timeout=1) as response:
                assert response.status == 200
                assert response.read().decode('utf-8') == expected
                return
        except (OSError, AssertionError) as error:
            last_error = error
            time.sleep(0.05)
    raise AssertionError('Site HTTP response did not match: %s' % last_error)


def remember_process(name):
    report = status(name)
    runtime = report['status']
    if runtime.get('pid') and runtime.get('state'):
        owned[name] = {
            'pid': runtime['pid'],
            'started': runtime['state'].get('process_started_at'),
        }
    return report


def still_owned(record):
    pid = record['pid']
    try:
        marker = subprocess.check_output(
            ['/bin/ps', '-p', str(pid), '-o', 'lstart='], text=True,
            stderr=subprocess.DEVNULL,
        ).strip()
        return bool(record['started']) and marker == record['started'] and os.getpgid(pid) == pid
    except (ProcessLookupError, subprocess.CalledProcessError):
        return False


def cleanup():
    failures = []
    for name in created:
        for candidate in [installed, previous, old_source / 'dist/Preside.app']:
            if not cli_path(candidate).exists():
                continue
            try:
                cli(['stop', '--', name], candidate, check=False)
            except (OSError, subprocess.TimeoutExpired):
                pass
        record = owned.get(name)
        if record and still_owned(record):
            os.killpg(record['pid'], signal.SIGTERM)
            deadline = time.monotonic() + 2
            while still_owned(record) and time.monotonic() < deadline:
                time.sleep(0.05)
            if still_owned(record):
                os.killpg(record['pid'], signal.SIGKILL)
                time.sleep(0.1)
            if still_owned(record):
                failures.append(name)
    if failures:
        raise RuntimeError('Could not clean up %s; isolated state retained at %s' % (failures, workspace))
    shutil.rmtree(workspace)


try:
    current_source = current_app
    current_app = workspace / 'Current Preside.app'
    subprocess.run(['ditto', '--norsrc', str(current_source), str(current_app)], check=True)
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(current_app)], check=True)
    archive = workspace / 'prior.tar'
    with archive.open('wb') as output:
        subprocess.run(['git', 'archive', '--format=tar', prior_ref], stdout=output, check=True)
    subprocess.run(['tar', '-xf', str(archive), '-C', str(old_source)], check=True)
    subprocess.run(['./scripts/build-macos.sh'], cwd=old_source, check=True)
    installed.parent.mkdir(parents=True)
    subprocess.run(['ditto', '--norsrc', str(old_source / 'dist/Preside.app'), str(installed)], check=True)
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(installed)], check=True)

    for arguments in [['path'], ['path', '--state']]:
        pathlib.Path(cli(arguments).stdout.strip()).resolve().relative_to(home.resolve())
    fixtures = []
    for name, kind in [('upgrade-static', 'static'), ('upgrade-command', 'command')]:
        directory = workspace / ('%s site files' % kind)
        directory.mkdir()
        body = '<!doctype html><title>%s</title><p>survives an app upgrade</p>' % name
        (directory / 'index.html').write_text(body)
        port = available_port()
        arguments = ['add', '--dir', str(directory), '--port', str(port)]
        if kind == 'static':
            arguments += ['--static']
        else:
            command = 'exec %s -m http.server "$PORT" --bind 127.0.0.1' % shlex.quote(sys.executable)
            arguments += ['--command', command, '--readiness', 'port']
        cli(arguments + ['--', name])
        created.append(name)
        result = cli(['run', '--detach', '--', name], check=False)
        report = remember_process(name)
        if result.returncode:
            raise RuntimeError('Could not start %s: %s\n%s' % (name, result.stdout, result.stderr))
        assert report['status']['identity_matches'], report
        assert report['status']['process_alive'], report
        fetch(port, body)
        fixtures.append((name, port, body, report['status']['pid']))

    config_path = pathlib.Path(cli(['path']).stdout.strip())
    assert str(config_path).startswith(str(home) + os.sep)
    original_config = config_path.read_bytes()
    original_list = json.loads(cli(['list', '--json']).stdout)
    installed.rename(previous)
    subprocess.run(['ditto', '--norsrc', str(current_app), str(installed)], check=True)
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(installed)], check=True)
    assert config_path.read_bytes() == original_config
    assert json.loads(cli(['list', '--json']).stdout) == original_list

    for name, port, body, original_pid in fixtures:
        report = status(name)
        assert report['status']['pid'] == original_pid, report
        assert report['status']['identity_matches'], report
        assert report['status']['process_alive'], report
        fetch(port, body)
        cli(['stop', '--', name])
        stopped = status(name)
        assert not stopped['status']['process_alive'], stopped
        assert not stopped['status']['identity_matches'], stopped
        with socket.socket() as connection:
            connection.settimeout(1)
            assert connection.connect_ex(('127.0.0.1', port)) != 0
    assert config_path.read_bytes() == original_config
    result_report = {
        'result': 'passed', 'prior_commit': prior_ref, 'current_checkout': current_ref,
        'prior_build_profile_strip': os.environ.get('CARGO_PROFILE_RELEASE_STRIP', 'toolchain default'),
        'current_cli_sha256': hashlib.sha256(cli_path(current_app).read_bytes()).hexdigest(),
        'sites': [fixture[0] for fixture in fixtures],
        'verified': ['unchanged configuration', 'unchanged PIDs', 'verified identities',
                     'HTTP before and after replacement', 'new CLI stop', 'closed ports'],
        'scope': 'local development app replacement; not signed release or clean-Mac acceptance',
    }
finally:
    cleanup()
print(json.dumps(result_report, indent=2))
PY
