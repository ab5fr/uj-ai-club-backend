import os
import sys
import time
import json
import logging
import subprocess
import shutil
import hmac
import requests
import threading
from pathlib import Path
from flask import Flask, request, jsonify, send_file
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler

from notebook_processor import (
    process_notebook_cells,
    process_notebook_file,
)

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger('grading_service')

EXCHANGE_ROOT = os.environ.get('EXCHANGE_ROOT', '/srv/nbgrader/exchange')
COURSE_ID = os.environ.get('COURSE_ID', 'ujaiclub')
WEBHOOK_URL = os.environ.get('GRADING_WEBHOOK_URL', 'http://backend:8000/webhooks/nbgrader/grade')
WEBHOOK_SECRET = os.environ.get('NBGRADER_WEBHOOK_SECRET', '')
DOCKER_SOCKET = os.environ.get('DOCKER_SOCKET', '/var/run/docker.sock')
GRADING_SERVICE_SECRET = os.environ.get('GRADING_SERVICE_SECRET', '')

app = Flask(__name__)

@app.before_request
def require_internal_auth():
    """All endpoints except health require a shared secret from the Rust API."""
    if request.path == '/health':
        return None

    if not GRADING_SERVICE_SECRET:
        return jsonify({'error': 'Grading service secret is not configured'}), 503

    provided = request.headers.get('X-Grading-Service-Secret', '')
    if not hmac.compare_digest(provided, GRADING_SERVICE_SECRET):
        return jsonify({'error': 'Unauthorized'}), 401

    return None

class SubmissionHandler(FileSystemEventHandler):
    """Handles new submission events."""

    def __init__(self):
        super().__init__()
        self.processed = set()

    def on_created(self, event):
        """Called when a new file or directory is created."""
        if event.is_directory:
            return

        path = Path(event.src_path)
        if path.suffix == '.ipynb' and 'submitted' in str(path):
            self.process_submission(path)

    def process_submission(self, submission_path):
        """Process a new submission."""
        if str(submission_path) in self.processed:
            return
        self.processed.add(str(submission_path))

        logger.info(f"Processing submission: {submission_path}")

        try:
            parts = submission_path.parts

            submitted_idx = parts.index('submitted')
            student_id = parts[submitted_idx + 1]
            assignment_name = parts[submitted_idx + 2]

            logger.info(f"Student: {student_id}, Assignment: {assignment_name}")

            result = self.run_autograder(student_id, assignment_name)

            if result:
                self.report_grade(student_id, assignment_name, result)

        except Exception as e:
            logger.error(f"Error processing submission: {e}")

    def run_autograder(self, student_id, assignment_name):
        """Run nbgrader autograde on a submission."""
        try:
            self.restore_nbgrader_metadata(student_id, assignment_name)

            cmd = [
                'nbgrader', 'autograde',
                assignment_name,
                '--student', student_id,
                '--force',
            ]

            logger.info(f"Running: {' '.join(cmd)}")

            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=300
            )

            if result.returncode != 0:
                logger.error(f"Autograde failed: {result.stderr}")
                return None

            grades = self.get_grades(student_id, assignment_name)
            return grades

        except subprocess.TimeoutExpired:
            logger.error("Autograding timed out")
            return None
        except Exception as e:
            logger.error(f"Autograding error: {e}")
            return None

    def get_grades(self, student_id, assignment_name):
        """Get grades from nbgrader for a submission."""
        return self.get_grades_from_db(student_id, assignment_name)

    def get_grades_from_db(self, student_id, assignment_name):
        """Get grades directly from nbgrader SQLite database."""
        import sqlite3

        db_path = '/data/nbgrader.db'
        if not os.path.exists(db_path):
            logger.error("nbgrader database not found")
            return None

        try:
            conn = sqlite3.connect(db_path)
            cursor = conn.cursor()

            cursor.execute('''
                SELECT
                    COALESCE(SUM(COALESCE(g.auto_score, 0) + COALESCE(g.manual_score, 0)), 0) as total_score,
                    COALESCE(SUM(gc.max_score), 0) as max_score
                FROM grade g
                JOIN submitted_notebook sn ON g.notebook_id = sn.id
                JOIN submitted_assignment sa ON sn.assignment_id = sa.id
                JOIN assignment a ON sa.assignment_id = a.id
                JOIN student s ON sa.student_id = s.id
                JOIN grade_cells gc ON g.cell_id = gc.id
                WHERE a.name = ? AND s.id = ?
            ''', (assignment_name, student_id))

            result = cursor.fetchone()
            conn.close()

            if result:
                score = result[0] if result[0] is not None else 0
                max_score = result[1] if result[1] is not None else 100
                logger.info(f"Got grades from DB: score={score}, max_score={max_score}")
                return {
                    'score': score,
                    'max_score': max_score
                }

            return None

        except Exception as e:
            logger.error(f"Database error: {e}")
            return None

    def report_grade(self, student_id, assignment_name, grades):
        """Report grades to the main application via webhook."""
        if not WEBHOOK_SECRET:
            logger.warning("NBGRADER_WEBHOOK_SECRET not set, using empty secret")

        payload = {
            'assignmentName': assignment_name,
            'studentId': student_id,
            'score': grades['score'],
            'maxScore': grades['max_score'],
            'webhookSecret': WEBHOOK_SECRET,
            'timestamp': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())
        }

        logger.info(f"Reporting grade to {WEBHOOK_URL}: {payload}")

        try:
            response = requests.post(
                WEBHOOK_URL,
                json=payload,
                headers={'Content-Type': 'application/json'},
                timeout=30
            )

            if response.status_code == 200:
                logger.info(f"Grade reported successfully: {response.json()}")
            else:
                logger.error(f"Failed to report grade: {response.status_code} - {response.text}")

        except Exception as e:
            logger.error(f"Error reporting grade: {e}")

    def restore_nbgrader_metadata(self, student_id, assignment_name):
        """
        Restore nbgrader metadata to a submitted notebook from the source notebook.
        This is necessary because students may lose metadata when editing in JupyterHub.
        """
        import json

        course_dir = '/srv/nbgrader/course'

        submitted_dir = Path(course_dir) / 'submitted' / student_id / assignment_name
        source_dir = Path(course_dir) / 'source' / assignment_name

        if not submitted_dir.exists():
            logger.warning(f"Submitted directory not found: {submitted_dir}")
            return

        if not source_dir.exists():
            logger.warning(f"Source directory not found: {source_dir}")
            return

        for submitted_nb_path in submitted_dir.glob('*.ipynb'):
            source_nb_path = source_dir / submitted_nb_path.name

            if not source_nb_path.exists():
                logger.warning(f"Source notebook not found: {source_nb_path}")
                continue

            try:
                with open(source_nb_path, 'r', encoding='utf-8') as f:
                    source_nb = json.load(f)

                with open(submitted_nb_path, 'r', encoding='utf-8') as f:
                    submitted_nb = json.load(f)

                source_metadata = {}
                for cell in source_nb.get('cells', []):
                    cell_id = cell.get('id')
                    if cell_id and 'nbgrader' in cell.get('metadata', {}):
                        source_metadata[cell_id] = cell['metadata']['nbgrader']

                restored_count = 0
                for cell in submitted_nb.get('cells', []):
                    cell_id = cell.get('id')
                    if cell_id and cell_id in source_metadata:
                        if 'metadata' not in cell:
                            cell['metadata'] = {}
                        cell['metadata']['nbgrader'] = source_metadata[cell_id]
                        restored_count += 1

                with open(submitted_nb_path, 'w', encoding='utf-8') as f:
                    json.dump(submitted_nb, f, indent=1)

                logger.info(f"Restored nbgrader metadata for {restored_count} cells in {submitted_nb_path.name}")

            except Exception as e:
                logger.error(f"Error restoring metadata for {submitted_nb_path}: {e}")

submission_handler = SubmissionHandler()

def find_user_container(student_id):
    """Find a running JupyterHub container for the given student username."""
    import docker

    client = docker.from_env()
    logger.info(f"Looking for container with JUPYTERHUB_USER={student_id}")

    try:
        containers = client.containers.list(filters={'name': 'ujaiclub'})
        for container in containers:
            env_vars = container.attrs.get('Config', {}).get('Env', [])
            for env in env_vars:
                if env == f'JUPYTERHUB_USER={student_id}':
                    logger.info(f"Found container {container.name} for user {student_id}")
                    return container
    except Exception as e:
        logger.info(f"Error searching containers: {e}")

    logger.info(f"No running container found for user: {student_id}")
    return None

def docker_escape(name):
    """
    Match dockerspawner's escape() so volume names align with
    'jupyterhub-user-{username}' mounts (e.g. '_' -> '-5f').
    """
    safe = set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-")
    escape_char = "-"
    out = []
    for c in name:
        if c in safe:
            out.append(c)
        else:
            out.append(escape_char)
            out.append(f"{ord(c):02x}")
    return "".join(out)

def user_work_volume_name(student_id):
    """Persistent Docker volume mounted at /home/jovyan/work for this user."""
    return f"jupyterhub-user-{docker_escape(student_id)}"

def workspace_has_notebook(student_id, notebook_filename):
    """Return True if the notebook already exists in the user's work volume/container."""
    import docker

    container = find_user_container(student_id)
    if container:
        exit_code, _ = container.exec_run(
            ["test", "-f", f"/home/jovyan/work/{notebook_filename}"]
        )
        return exit_code == 0

    client = docker.from_env()
    volume_name = user_work_volume_name(student_id)
    try:
        client.containers.run(
            "alpine:latest",
            ["test", "-f", f"/data/{notebook_filename}"],
            volumes={volume_name: {"bind": "/data", "mode": "ro"}},
            remove=True,
            detach=False,
        )
        return True
    except docker.errors.ContainerError:
        return False
    except Exception as e:
        logger.info(f"Could not check volume {volume_name} for existing notebook: {e}")
        return False

def ensure_user_volume_writable(volume_name):
    """
    Docker creates empty named volumes as root:root 755. Jupyter runs as
    jovyan (1000:100) and needs write access on the work dir for saves,
    autosave, and checkpoints. Match DockerSpawner's usual ownership.
    """
    import docker

    client = docker.from_env()
    client.containers.run(
        "alpine:latest",
        [
            "sh",
            "-c",
            "chown -R 1000:100 /data && chmod 2775 /data && "
            "mkdir -p /data/.ipynb_checkpoints && "
            "chown -R 1000:100 /data/.ipynb_checkpoints && "
            "chmod 2775 /data/.ipynb_checkpoints",
        ],
        volumes={volume_name: {"bind": "/data", "mode": "rw"}},
        remove=True,
        detach=False,
    )

def write_notebook_to_user_volume(student_id, notebook_filename, notebook_bytes):
    """
    Stage a notebook onto the user's persistent work volume even if their
    Jupyter container is not running yet. DockerSpawner mounts this volume at
    /home/jovyan/work on spawn, so the file is present on first open.
    """
    import docker
    import tarfile
    import io

    client = docker.from_env()
    volume_name = user_work_volume_name(student_id)

    tar_stream = io.BytesIO()
    with tarfile.open(fileobj=tar_stream, mode="w") as tar:
        tarinfo = tarfile.TarInfo(name=notebook_filename)
        tarinfo.size = len(notebook_bytes)
        tarinfo.uid = 1000
        tarinfo.gid = 100
        tarinfo.mode = 0o644
        tar.addfile(tarinfo, io.BytesIO(notebook_bytes))
    tar_stream.seek(0)

    temp = client.containers.create(
        "alpine:latest",
        command=["true"],
        volumes={volume_name: {"bind": "/data", "mode": "rw"}},
    )
    try:
        temp.put_archive("/data", tar_stream)
        logger.info(
            f"Staged {notebook_filename} onto volume {volume_name} for user {student_id}"
        )
    finally:
        try:
            temp.remove(force=True)
        except Exception:
            pass

    ensure_user_volume_writable(volume_name)

def write_bytes_to_user_workspace(student_id, relative_path, content_bytes):
    """Write an arbitrary file into the user's work volume (and running container if any)."""
    import docker
    import tarfile
    import io

    container = find_user_container(student_id)
    tar_stream = io.BytesIO()
    with tarfile.open(fileobj=tar_stream, mode="w") as tar:
        tarinfo = tarfile.TarInfo(name=relative_path)
        tarinfo.size = len(content_bytes)
        tarinfo.uid = 1000
        tarinfo.gid = 100
        tarinfo.mode = 0o644
        tar.addfile(tarinfo, io.BytesIO(content_bytes))
    tar_stream.seek(0)

    if container:
        parent = str(Path(relative_path).parent)
        if parent not in ("", "."):
            container.exec_run(["mkdir", "-p", f"/home/jovyan/work/{parent}"], user="0")
        container.put_archive("/home/jovyan/work", tar_stream)
        return

    write_notebook_to_user_volume(student_id, relative_path, content_bytes)

def write_spawn_config(student_id, network_disabled, cpu_limit, memory_limit):
    """Persist per-challenge spawner settings for JupyterHub pre_spawn_hook."""
    import json as json_module

    payload = json_module.dumps(
        {
            "networkDisabled": bool(network_disabled),
            "cpuLimit": cpu_limit,
            "memoryLimit": memory_limit,
        }
    ).encode("utf-8")
    write_bytes_to_user_workspace(student_id, ".ujaiclub_spawn.json", payload)
    logger.info(
        f"Wrote spawn config for {student_id}: networkDisabled={network_disabled}, "
        f"cpuLimit={cpu_limit}, memoryLimit={memory_limit}"
    )

def remove_notebook_from_workspace(student_id, notebook_filename):
    """Delete a notebook (and its checkpoint) so a new attempt can start fresh."""
    import docker

    paths = [
        f"/home/jovyan/work/{notebook_filename}",
        f"/home/jovyan/work/.ipynb_checkpoints/{Path(notebook_filename).stem}-checkpoint.ipynb",
    ]

    container = find_user_container(student_id)
    if container:
        for path in paths:
            container.exec_run(["rm", "-f", path], user="0")
        logger.info(f"Removed existing notebook files for {student_id} in running container")
        return

    client = docker.from_env()
    volume_name = user_work_volume_name(student_id)
    try:
        client.containers.run(
            "alpine:latest",
            [
                "sh",
                "-c",
                f"rm -f /data/{notebook_filename} "
                f"/data/.ipynb_checkpoints/{Path(notebook_filename).stem}-checkpoint.ipynb",
            ],
            volumes={volume_name: {"bind": "/data", "mode": "rw"}},
            remove=True,
            detach=False,
        )
        logger.info(f"Removed existing notebook files for {student_id} from volume {volume_name}")
    except Exception as e:
        logger.warning(f"Could not remove existing notebook for {student_id}: {e}")

def force_save_notebooks_in_container(container, notebook_filename=None):
    """
    Force-save open notebooks via the Jupyter Contents API inside a user container.
    Reads the in-memory notebook model (including unsaved edits) and writes it to disk.
    """
    import json

    explicit_repr = repr(notebook_filename)
    script = f'''
import json
import subprocess
import sys
import urllib.parse
import urllib.request

EXPLICIT = {explicit_repr}

def get_base_url():
    try:
        out = subprocess.check_output(
            ['jupyter', 'server', 'list', '--json'],
            text=True,
            timeout=10,
        )
        servers = json.loads(out)
        if isinstance(servers, list):
            for server in servers:
                url = server.get('url', '')
                if url:
                    return url.rstrip('/')
    except Exception as exc:
        print(f"server list failed: {{exc}}", file=sys.stderr)
    return 'http://127.0.0.1:8888'

def api(base, path, method='GET', data=None):
    url = base + path
    headers = {{}}
    body = None
    if data is not None:
        headers['Content-Type'] = 'application/json'
        body = json.dumps(data).encode()
    req = urllib.request.Request(url, data=body, headers=headers, method=method)
    with urllib.request.urlopen(req, timeout=15) as resp:
        if resp.status == 204:
            return None
        raw = resp.read()
        return json.loads(raw) if raw else None

base = get_base_url()
paths = []

try:
    sessions = api(base, '/api/sessions') or []
    for session in sessions:
        path = session.get('path') or (session.get('notebook') or {{}}).get('path')
        if path and path not in paths:
            paths.append(path)
except Exception as exc:
    print(f"sessions failed: {{exc}}", file=sys.stderr)

if EXPLICIT and EXPLICIT not in paths:
    paths.append(EXPLICIT)

saved = []
for path in paths:
    try:
        encoded = urllib.parse.quote(path, safe='/')
        notebook = api(base, f'/api/contents/{{encoded}}?content=1&type=notebook')
        api(
            base,
            f'/api/contents/{{encoded}}',
            method='PUT',
            data={{
                'type': 'notebook',
                'format': 'json',
                'content': notebook['content'],
            }},
        )
        saved.append(path)
    except Exception as exc:
        print(f"failed to save {{path}}: {{exc}}", file=sys.stderr)

print(json.dumps({{'saved': saved, 'base': base}}))
'''

    exit_code, output = container.exec_run(['python3', '-c', script])
    stdout = output.decode('utf-8', errors='replace').strip() if output else ''

    if exit_code != 0:
        logger.warning(
            f"Notebook save script exited {exit_code} for {container.name}: {stdout}"
        )
        return False, stdout or 'save script failed'

    try:
        result = json.loads(stdout.splitlines()[-1])
        saved = result.get('saved', [])
        logger.info(f"Force-saved notebooks in {container.name}: {saved}")
        return True, saved
    except Exception as e:
        logger.warning(f"Could not parse save script output for {container.name}: {e}")
        return bool(stdout), stdout

def save_user_notebook(student_id, notebook_filename=None):
    """Save notebooks for a user, whether or not a specific filename is provided."""
    container = find_user_container(student_id)
    if not container:
        return True, 'No running container (notebook may already be on disk)'

    return force_save_notebooks_in_container(container, notebook_filename)

def copy_notebook_from_user(student_id, assignment_name, notebook_filename):
    """
    Copy a notebook from a user's JupyterHub container or volume to the exchange directory.
    First tries to copy from running container, then falls back to volume.
    """
    import docker

    try:
        client = docker.from_env()
        container = find_user_container(student_id)

        course_dir = '/srv/nbgrader/course'
        dest_dir = Path(course_dir) / 'submitted' / student_id / assignment_name
        dest_dir.mkdir(parents=True, exist_ok=True)

        if container:
            src_path = f"/home/jovyan/work/{notebook_filename}"

            try:
                bits, stat = container.get_archive(src_path)

                import tarfile
                import io

                tar_stream = io.BytesIO()
                for chunk in bits:
                    tar_stream.write(chunk)
                tar_stream.seek(0)

                with tarfile.open(fileobj=tar_stream) as tar:
                    for member in tar.getmembers():
                        if member.name.endswith('.ipynb'):
                            f = tar.extractfile(member)
                            if f:
                                content = f.read()
                                dest_file = dest_dir / notebook_filename
                                with open(dest_file, 'wb') as out:
                                    out.write(content)
                                logger.info(f"Copied notebook from container to: {dest_file}")
                                return True, str(dest_file)

                return False, "Failed to extract notebook from container archive"

            except docker.errors.NotFound:
                logger.info(f"Notebook not found in container, trying volume")

        volume_name = user_work_volume_name(student_id)
        logger.info(f"Trying to access volume: {volume_name}")

        try:
            temp_container = client.containers.run(
                'alpine:latest',
                'cat /data/' + notebook_filename,
                volumes={volume_name: {'bind': '/data', 'mode': 'ro'}},
                remove=True,
                detach=False,
                stdout=True,
                stderr=True
            )

            if temp_container:
                dest_file = dest_dir / notebook_filename
                with open(dest_file, 'wb') as out:
                    out.write(temp_container)
                logger.info(f"Copied notebook from volume to: {dest_file}")
                return True, str(dest_file)

        except Exception as vol_err:
            logger.error(f"Failed to access volume {volume_name}: {vol_err}")

        return False, f"Could not find notebook. Please make sure your JupyterHub session is active and you have saved your work."

    except Exception as e:
        logger.error(f"Error copying notebook: {e}")
        return False, str(e)

@app.route('/health', methods=['GET'])
def health_check():
    """Health check endpoint."""
    return jsonify({'status': 'healthy', 'service': 'grading'})

@app.route('/prepare-notebook/<student_id>/<assignment_name>', methods=['POST'])
def prepare_notebook_for_user(student_id, assignment_name):
    """
    Prepare a notebook for a user by copying it to their JupyterHub workspace.
    This should be called when a user starts a challenge.

    If the user container is not running yet, the notebook is staged onto their
    persistent Docker volume so it exists when JupyterHub spawns.

    Expected JSON payload:
    {
        "notebookPath": "uploads/notebooks/uuid_filename.ipynb",
        "notebookFilename": "original_filename.ipynb",
        "networkDisabled": true,
        "cpuLimit": 0.5,
        "memoryLimit": "512M",
        "forceFresh": false
    }
    """
    logger.info(f"Preparing notebook for user {student_id}, assignment {assignment_name}")

    data = request.get_json() or {}
    notebook_path = data.get('notebookPath')
    notebook_filename = data.get('notebookFilename')
    force_fresh = bool(data.get('forceFresh', False))
    network_disabled = data.get('networkDisabled', True)
    if isinstance(network_disabled, str):
        network_disabled = network_disabled.lower() in ('1', 'true', 'yes')
    cpu_limit = data.get('cpuLimit', 0.5)
    memory_limit = data.get('memoryLimit', '512M')

    if not notebook_path or not notebook_filename:
        return jsonify({
            'success': False,
            'error': 'notebookPath and notebookFilename are required'
        }), 400

    try:
        write_spawn_config(student_id, network_disabled, cpu_limit, memory_limit)

        if workspace_has_notebook(student_id, notebook_filename) and not force_fresh:
            logger.info(
                f"Notebook {notebook_filename} already exists for {student_id}; skipping copy"
            )
            return jsonify({
                'success': True,
                'message': f'Notebook {notebook_filename} already present for user {student_id}',
                'skipped': True,
            })

        if force_fresh and workspace_has_notebook(student_id, notebook_filename):
            remove_notebook_from_workspace(student_id, notebook_filename)

        source_path = f"/srv/notebooks/{notebook_path.replace('uploads/', '')}"

        if not os.path.exists(source_path):
            source_path = f"/srv/notebooks/notebooks/{os.path.basename(notebook_path)}"

        if not os.path.exists(source_path):
            return jsonify({
                'success': False,
                'error': f'Source notebook not found at {source_path}'
            }), 404

        import json as json_module
        import tarfile
        import io

        with open(source_path, 'r', encoding='utf-8') as f:
            notebook = json_module.load(f)

        process_notebook_cells(notebook)
        notebook_bytes = json_module.dumps(notebook, indent=1).encode('utf-8')

        container = find_user_container(student_id)

        if container:
            tar_stream = io.BytesIO()
            with tarfile.open(fileobj=tar_stream, mode='w') as tar:
                tarinfo = tarfile.TarInfo(name=notebook_filename)
                tarinfo.size = len(notebook_bytes)
                tarinfo.uid = 1000
                tarinfo.gid = 100
                tarinfo.mode = 0o644
                tar.addfile(tarinfo, io.BytesIO(notebook_bytes))
            tar_stream.seek(0)
            container.put_archive('/home/jovyan/work', tar_stream)
            logger.info(
                f"Successfully copied {notebook_filename} to user {student_id}'s running container"
            )
        else:
            write_notebook_to_user_volume(student_id, notebook_filename, notebook_bytes)

        return jsonify({
            'success': True,
            'message': f'Notebook {notebook_filename} prepared for user {student_id}',
            'fresh': force_fresh,
        })

    except Exception as e:
        logger.error(f"Error preparing notebook: {e}")
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500

@app.route('/stop-user/<student_id>', methods=['POST'])
def stop_user_server(student_id):
    """
    Stop a student's JupyterHub single-user container via Docker.
    Used as a reliable shutdown path after submit when Hub RBAC may block DELETE.
    """
    logger.info(f"Stopping Jupyter container for user {student_id}")
    try:
        container = find_user_container(student_id)
        if not container:
            return jsonify({
                'success': True,
                'message': f'No running container for {student_id}',
                'stopped': False,
            })

        name = container.name
        container.stop(timeout=10)
        try:
            container.remove(force=True)
        except Exception as remove_err:
            logger.warning(f"Container {name} stopped but remove failed: {remove_err}")

        logger.info(f"Stopped and removed container {name} for user {student_id}")
        return jsonify({
            'success': True,
            'message': f'Stopped container {name}',
            'stopped': True,
        })
    except Exception as e:
        logger.error(f"Error stopping user container for {student_id}: {e}")
        return jsonify({'success': False, 'error': str(e)}), 500

@app.route('/save-notebook/<student_id>', methods=['POST'])
def save_notebook(student_id):
    """
    Force-save the student's open notebook(s) to disk before closing Jupyter.
    """
    data = request.get_json() or {}
    notebook_filename = data.get('notebookFilename')

    logger.info(f"Saving notebook(s) for user {student_id}")

    success, result = save_user_notebook(student_id, notebook_filename)

    return jsonify({
        'success': success,
        'message': 'Notebook saved' if success else 'Notebook save failed',
        'saved': result if isinstance(result, list) else [],
        'detail': result if isinstance(result, str) else None,
    }), 200 if success else 500

@app.route('/submit/<student_id>/<assignment_name>', methods=['POST'])
def submit_for_grading(student_id, assignment_name):
    """
    Submit a notebook for grading.
    Copies the notebook from user's container to course/submitted and triggers grading.
    """
    logger.info(f"Received submission request: student={student_id}, assignment={assignment_name}")

    data = request.get_json() or {}
    notebook_filename = data.get('notebookFilename', f"{assignment_name}.ipynb")
    notebook_path = data.get('notebookPath', '')

    source_dir = Path('/srv/nbgrader/course/source') / assignment_name
    if not source_dir.exists() and notebook_path:
        logger.info(f"Setting up source assignment {assignment_name} before grading")
        try:
            source_notebook = f"/srv/notebooks/{notebook_path.replace('uploads/', '')}"
            if not os.path.exists(source_notebook):
                source_notebook = f"/srv/notebooks/notebooks/{os.path.basename(notebook_path)}"

            if os.path.exists(source_notebook):
                setup_nbgrader_assignment(source_notebook, assignment_name)
                logger.info(f"Source assignment {assignment_name} set up successfully")
            else:
                logger.warning(f"Could not find source notebook to set up assignment")
        except Exception as e:
            logger.error(f"Failed to set up source assignment: {e}")

    save_success, save_result = save_user_notebook(student_id, notebook_filename)
    if not save_success:
        logger.warning(f"Pre-submit save failed for {student_id}: {save_result}")

    success, result = copy_notebook_from_user(student_id, assignment_name, notebook_filename)

    if not success:
        return jsonify({
            'success': False,
            'error': result
        }), 400

    def grade_async():
        time.sleep(1)
        submission_path = Path(result)
        submission_handler.process_submission(submission_path)

    threading.Thread(target=grade_async, daemon=True).start()

    return jsonify({
        'success': True,
        'message': f'Submission received, grading started',
        'path': result
    })

@app.route('/grade/<student_id>/<assignment_name>', methods=['POST'])
def trigger_grading(student_id, assignment_name):
    """
    Directly trigger grading for an existing submission.
    """
    logger.info(f"Received grading request: student={student_id}, assignment={assignment_name}")

    submission_dir = Path('/srv/nbgrader/course') / 'submitted' / student_id / assignment_name

    if not submission_dir.exists():
        return jsonify({
            'success': False,
            'error': 'Submission not found'
        }), 404

    notebooks = list(submission_dir.glob('*.ipynb'))
    if not notebooks:
        return jsonify({
            'success': False,
            'error': 'No notebook found in submission'
        }), 404

    def grade_async():
        submission_handler.process_submission(notebooks[0])

    threading.Thread(target=grade_async, daemon=True).start()

    return jsonify({
        'success': True,
        'message': 'Grading started'
    })

@app.route('/submissions/<student_id>/<assignment_name>/notebook', methods=['GET'])
def get_submitted_notebook(student_id, assignment_name):
    """
    Return the latest submitted notebook file for a student/assignment.

    Query params:
      - download=1 to force attachment download
    """
    submission_dir = Path('/srv/nbgrader/course') / 'submitted' / student_id / assignment_name

    if not submission_dir.exists():
        return jsonify({
            'success': False,
            'error': 'Submission not found'
        }), 404

    notebooks = list(submission_dir.glob('*.ipynb'))
    if not notebooks:
        return jsonify({
            'success': False,
            'error': 'No notebook found in submission'
        }), 404

    latest_notebook = max(notebooks, key=lambda p: p.stat().st_mtime)
    as_attachment = str(request.args.get('download', '0')).lower() in ('1', 'true', 'yes')

    return send_file(
        latest_notebook,
        mimetype='application/x-ipynb+json',
        as_attachment=as_attachment,
        download_name=latest_notebook.name,
        conditional=True,
    )

@app.route('/setup-assignment/<assignment_name>', methods=['POST'])
def setup_assignment(assignment_name):
    """
    Set up an nbgrader assignment from a source notebook.
    This endpoint is called by the admin to sync a notebook to nbgrader's source directory.

    Expected JSON payload:
    {
        "notebookPath": "/srv/notebooks/notebooks/uuid_filename.ipynb",
        "assignmentName": "week5_challenge",
        "maxPoints": 100
    }
    """
    logger.info(f"Setting up assignment: {assignment_name}")

    data = request.get_json() or {}
    notebook_path = data.get('notebookPath')
    max_points = data.get('maxPoints', 100)

    if not notebook_path:
        return jsonify({
            'success': False,
            'error': 'notebookPath is required'
        }), 400

    try:
        result = setup_nbgrader_assignment(notebook_path, assignment_name)
        return jsonify({
            'success': True,
            'message': f'Assignment {assignment_name} set up successfully',
            'sourcePath': result.get('source_path'),
            'releasePath': result.get('release_path')
        })
    except Exception as e:
        logger.error(f"Failed to setup assignment: {e}")
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500

@app.route('/cleanup-assignment/<assignment_name>', methods=['DELETE'])
def cleanup_assignment(assignment_name):
    """
    Remove all nbgrader directories for an assignment when a challenge is deleted.
    Cleans up: source, release, submitted, autograded, and feedback directories.
    """
    logger.info(f"Cleaning up assignment: {assignment_name}")

    course_dir = Path('/srv/nbgrader/course')
    removed = []
    errors = []

    for subdir_name in ['source', 'release']:
        d = course_dir / subdir_name / assignment_name
        if d.exists():
            try:
                shutil.rmtree(str(d))
                removed.append(str(d))
                logger.info(f"Removed directory: {d}")
            except Exception as e:
                errors.append(f"{d}: {e}")
                logger.error(f"Failed to remove {d}: {e}")

    for subdir_name in ['submitted', 'autograded', 'feedback']:
        subdir = course_dir / subdir_name
        if subdir.exists():
            for student_dir in subdir.iterdir():
                assignment_dir = student_dir / assignment_name
                if assignment_dir.exists():
                    try:
                        shutil.rmtree(str(assignment_dir))
                        removed.append(str(assignment_dir))
                        logger.info(f"Removed student directory: {assignment_dir}")
                    except Exception as e:
                        errors.append(f"{assignment_dir}: {e}")
                        logger.error(f"Failed to remove {assignment_dir}: {e}")

    if errors:
        return jsonify({
            'success': False,
            'removed': removed,
            'errors': errors,
        }), 500

    return jsonify({
        'success': True,
        'message': f'Assignment {assignment_name} cleaned up',
        'removed': removed,
    })

def setup_nbgrader_assignment(source_notebook_path, assignment_name, course_dir='/srv/nbgrader/course'):
    """
    Set up an nbgrader assignment from a source notebook.

    1. Copy the source notebook to nbgrader's source directory
    2. Generate a student version (removes solutions and hidden tests)
    3. Place the student version in the release directory

    Args:
        source_notebook_path: Path to the source notebook
        assignment_name: Name of the assignment
        course_dir: Base directory for nbgrader course

    Returns:
        dict with source_path and release_path
    """
    import json
    import shutil
    import re

    logger.info(f"Setting up nbgrader assignment: {assignment_name}")
    logger.info(f"Source notebook: {source_notebook_path}")

    if not os.path.exists(source_notebook_path):
        raise FileNotFoundError(f"Source notebook not found: {source_notebook_path}")

    source_dir = Path(course_dir) / 'source' / assignment_name
    release_dir = Path(course_dir) / 'release' / assignment_name

    source_dir.mkdir(parents=True, exist_ok=True)
    release_dir.mkdir(parents=True, exist_ok=True)

    original_filename = Path(source_notebook_path).name
    notebook_name = re.sub(r'^[a-f0-9-]{36}_', '', original_filename)

    logger.info(f"Notebook name (cleaned): {notebook_name}")

    source_dest = source_dir / notebook_name
    add_nbgrader_metadata(source_notebook_path, str(source_dest))
    logger.info(f"Copied source notebook with nbgrader metadata to: {source_dest}")

    try:
        cmd = [
            'nbgrader', 'generate_assignment', assignment_name,
            '--force',
            '--IncludeHeaderFooter.header=',
        ]
        logger.info(f"Running: {' '.join(cmd)}")
        result = subprocess.run(
            cmd,
            cwd=course_dir,
            capture_output=True,
            text=True,
            timeout=60
        )
        if result.returncode != 0:
            logger.warning(f"nbgrader generate_assignment output: {result.stdout}")
            logger.warning(f"nbgrader generate_assignment errors: {result.stderr}")
            logger.info("Falling back to manual release creation")
        else:
            logger.info(f"Successfully generated assignment {assignment_name}")
    except subprocess.TimeoutExpired:
        logger.warning(f"nbgrader generate_assignment timed out for {assignment_name}")
    except Exception as e:
        logger.warning(f"Failed to run nbgrader generate_assignment: {e}")

    release_dest = release_dir / notebook_name
    if not release_dest.exists():
        try:
            release_dir.mkdir(parents=True, exist_ok=True)
            process_notebook_for_students(str(source_dest), str(release_dest))
            logger.info(f"Created student version at: {release_dest}")
        except Exception as e:
            logger.error(f"Failed to create student version: {e}")
            import shutil
            shutil.copy2(str(source_dest), str(release_dest))
            logger.info(f"Copied source as fallback to: {release_dest}")

    if not release_dest.exists():
        raise FileNotFoundError(f"Failed to create release notebook at {release_dest}")

    return {
        'source_path': str(source_dest),
        'release_path': str(release_dest)
    }

def add_nbgrader_metadata(source_path, output_path):
    """
    Add nbgrader cell metadata to a notebook based on solution/test markers.
    This is required for nbgrader generate_assignment to work properly.
    """
    import json
    import hashlib

    try:
        with open(source_path, 'r', encoding='utf-8') as f:
            notebook = json.load(f)
    except Exception as e:
        logger.error(f"Error reading notebook: {e}")
        import shutil
        shutil.copy2(source_path, output_path)
        return

    cell_counter = 0
    total_points = 0

    for cell in notebook.get('cells', []):
        if isinstance(cell.get('source'), list):
            source = ''.join(cell.get('source', []))
        else:
            source = cell.get('source', '')

        if 'metadata' not in cell:
            cell['metadata'] = {}

        is_solution = ('### BEGIN SOLUTION' in source or '# BEGIN SOLUTION' in source)
        is_test = ('### BEGIN HIDDEN TESTS' in source or '# BEGIN HIDDEN TESTS' in source)
        has_assert = 'assert ' in source and not is_test

        if is_solution or is_test or has_assert:
            cell_counter += 1
            grade_id = f"cell_{cell_counter}"

            points = 10
            total_points += points

            nbgrader_meta = {
                "grade_id": grade_id,
                "locked": False,
                "schema_version": 3,
            }

            if is_solution:
                nbgrader_meta["solution"] = True
                nbgrader_meta["grade"] = False
                nbgrader_meta["task"] = False
                logger.info(f"Marked cell {cell_counter} as solution cell")
            elif is_test or has_assert:
                nbgrader_meta["solution"] = False
                nbgrader_meta["grade"] = True
                nbgrader_meta["points"] = points
                nbgrader_meta["task"] = False
                nbgrader_meta["locked"] = True
                logger.info(f"Marked cell {cell_counter} as test cell with {points} points")

            cell['metadata']['nbgrader'] = nbgrader_meta

    logger.info(f"Processed notebook with {cell_counter} graded cells, total {total_points} points")

    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(notebook, f, indent=1)

def process_notebook_for_students(source_path, output_path):
    """Process a source notebook to create a student version."""
    process_notebook_file(source_path, output_path)

def run_watcher():
    """Run the file watcher in a separate thread."""
    submitted_path = os.path.join(EXCHANGE_ROOT, COURSE_ID, 'submitted')
    os.makedirs(submitted_path, exist_ok=True)

    observer = Observer()
    observer.schedule(submission_handler, submitted_path, recursive=True)
    observer.start()

    logger.info(f"Watching for submissions in: {submitted_path}")

    return observer

def main():
    """Main entry point for the grading service."""
    logger.info("Starting grading service...")
    logger.info(f"Exchange root: {EXCHANGE_ROOT}")
    logger.info(f"Webhook URL: {WEBHOOK_URL}")

    watcher_thread = threading.Thread(target=run_watcher, daemon=True)
    watcher_thread.start()

    port = int(os.environ.get('GRADING_SERVICE_PORT', 9100))
    logger.info(f"Starting HTTP API on port {port}")
    app.run(host='0.0.0.0', port=port, threaded=True)

if __name__ == '__main__':
    main()
