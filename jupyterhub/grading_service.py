"""
Grading Service for UJ AI Club
HTTP API for triggering grading + watches for submissions, runs autograder, 
and reports grades to the main application.
"""

import os
import sys
import time
import json
import logging
import subprocess
import shutil
import requests
import threading
from pathlib import Path
from flask import Flask, request, jsonify
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger('grading_service')

# Configuration from environment
EXCHANGE_ROOT = os.environ.get('EXCHANGE_ROOT', '/srv/nbgrader/exchange')
COURSE_ID = os.environ.get('COURSE_ID', 'ujaiclub')
WEBHOOK_URL = os.environ.get('GRADING_WEBHOOK_URL', 'http://backend:8000/webhooks/nbgrader/grade')
WEBHOOK_SECRET = os.environ.get('NBGRADER_WEBHOOK_SECRET', '')
DOCKER_SOCKET = os.environ.get('DOCKER_SOCKET', '/var/run/docker.sock')

# Flask app for HTTP API
app = Flask(__name__)


class SubmissionHandler(FileSystemEventHandler):
    """Handles new submission events."""
    
    def __init__(self):
        super().__init__()
        self.processed = set()
    
    def on_created(self, event):
        """Called when a new file or directory is created."""
        if event.is_directory:
            return
        
        # Check if this is a submission file
        path = Path(event.src_path)
        if path.suffix == '.ipynb' and 'submitted' in str(path):
            self.process_submission(path)
    
    def process_submission(self, submission_path):
        """Process a new submission."""
        # Avoid processing the same file multiple times
        if str(submission_path) in self.processed:
            return
        self.processed.add(str(submission_path))
        
        logger.info(f"Processing submission: {submission_path}")
        
        try:
            # Parse submission path to get assignment and student info
            # Expected format: /exchange/ujaiclub/submitted/<student>/<assignment>/<notebook>.ipynb
            parts = submission_path.parts
            
            # Find the indices
            submitted_idx = parts.index('submitted')
            student_id = parts[submitted_idx + 1]
            assignment_name = parts[submitted_idx + 2]
            
            logger.info(f"Student: {student_id}, Assignment: {assignment_name}")
            
            # Run autograder
            result = self.run_autograder(student_id, assignment_name)
            
            if result:
                # Report grade to main application
                self.report_grade(student_id, assignment_name, result)
            
        except Exception as e:
            logger.error(f"Error processing submission: {e}")
    
    def run_autograder(self, student_id, assignment_name):
        """Run nbgrader autograde on a submission."""
        try:
            # First, restore nbgrader metadata on the submitted notebook
            # This is necessary because students' edits can strip the metadata
            self.restore_nbgrader_metadata(student_id, assignment_name)
            
            # Run autograde command
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
                timeout=300  # 5 minute timeout
            )
            
            if result.returncode != 0:
                logger.error(f"Autograde failed: {result.stderr}")
                return None
            
            # Get the grades from nbgrader database
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
        try:
            # Use nbgrader API to get grades
            cmd = [
                'nbgrader', 'export',
                '--to', 'json',
                '--assignment', assignment_name,
                '--student', student_id,
            ]
            
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=60
            )
            
            if result.returncode == 0 and result.stdout:
                grades_data = json.loads(result.stdout)
                
                # Calculate total score
                total_score = 0
                max_score = 0
                
                for grade in grades_data.get('grades', []):
                    if grade.get('score') is not None:
                        total_score += grade['score']
                    max_score += grade.get('max_score', 0)
                
                return {
                    'score': total_score,
                    'max_score': max_score,
                    'details': grades_data
                }
            
            # Fallback: try to parse grades from database directly
            return self.get_grades_from_db(student_id, assignment_name)
            
        except Exception as e:
            logger.error(f"Error getting grades: {e}")
            return None
    
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
            
            # Query for the latest submission grades
            # The grade table has auto_score and manual_score, not score
            # We need to join through submitted_notebook -> submitted_assignment -> assignment
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
        
        # Find the submitted notebook
        submitted_dir = Path(course_dir) / 'submitted' / student_id / assignment_name
        source_dir = Path(course_dir) / 'source' / assignment_name
        
        if not submitted_dir.exists():
            logger.warning(f"Submitted directory not found: {submitted_dir}")
            return
        
        if not source_dir.exists():
            logger.warning(f"Source directory not found: {source_dir}")
            return
        
        # Process each notebook in the submission
        for submitted_nb_path in submitted_dir.glob('*.ipynb'):
            source_nb_path = source_dir / submitted_nb_path.name
            
            if not source_nb_path.exists():
                logger.warning(f"Source notebook not found: {source_nb_path}")
                continue
            
            try:
                # Read source notebook to get metadata
                with open(source_nb_path, 'r', encoding='utf-8') as f:
                    source_nb = json.load(f)
                
                # Read submitted notebook
                with open(submitted_nb_path, 'r', encoding='utf-8') as f:
                    submitted_nb = json.load(f)
                
                # Create a mapping of cell_id to metadata from source
                source_metadata = {}
                for cell in source_nb.get('cells', []):
                    cell_id = cell.get('id')
                    if cell_id and 'nbgrader' in cell.get('metadata', {}):
                        source_metadata[cell_id] = cell['metadata']['nbgrader']
                
                # Restore metadata to submitted cells
                restored_count = 0
                for cell in submitted_nb.get('cells', []):
                    cell_id = cell.get('id')
                    if cell_id and cell_id in source_metadata:
                        if 'metadata' not in cell:
                            cell['metadata'] = {}
                        cell['metadata']['nbgrader'] = source_metadata[cell_id]
                        restored_count += 1
                
                # Write back the submitted notebook with restored metadata
                with open(submitted_nb_path, 'w', encoding='utf-8') as f:
                    json.dump(submitted_nb, f, indent=1)
                
                logger.info(f"Restored nbgrader metadata for {restored_count} cells in {submitted_nb_path.name}")
                
            except Exception as e:
                logger.error(f"Error restoring metadata for {submitted_nb_path}: {e}")


# Global submission handler for use in API
submission_handler = SubmissionHandler()


def copy_notebook_from_user(student_id, assignment_name, notebook_filename):
    """
    Copy a notebook from a user's JupyterHub container or volume to the exchange directory.
    First tries to copy from running container, then falls back to volume.
    """
    import docker
    
    try:
        client = docker.from_env()
        
        # Find the user's container by searching for JUPYTERHUB_USER env var
        # Container names are escaped by DockerSpawner, so we can't rely on simple name matching
        logger.info(f"Looking for container with JUPYTERHUB_USER={student_id}")
        
        container = None
        try:
            # Search all containers with ujaiclub prefix
            containers = client.containers.list(filters={'name': 'ujaiclub'})
            for c in containers:
                # Check if this container belongs to our user
                env_vars = c.attrs.get('Config', {}).get('Env', [])
                for env in env_vars:
                    if env == f'JUPYTERHUB_USER={student_id}':
                        container = c
                        logger.info(f"Found container {c.name} for user {student_id}")
                        break
                if container:
                    break
            
            if not container:
                logger.info(f"No running container found for user: {student_id}")
        except Exception as e:
            logger.info(f"Error searching containers: {e}")
            container = None
        
        # Create destination directory in COURSE directory (not exchange)
        # nbgrader autograde expects submissions in /srv/nbgrader/course/submitted/
        course_dir = '/srv/nbgrader/course'
        dest_dir = Path(course_dir) / 'submitted' / student_id / assignment_name
        dest_dir.mkdir(parents=True, exist_ok=True)
        
        # If container is running, try to get file from it
        if container:
            # Source path in user container
            src_path = f"/home/jovyan/work/{notebook_filename}"
            
            try:
                bits, stat = container.get_archive(src_path)
                
                # Extract and save the file
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
        
        # Fallback: Try to get from user's persistent volume
        # Volume name format: jupyterhub-user-{username}
        volume_name = f"jupyterhub-user-{student_id}"
        logger.info(f"Trying to access volume: {volume_name}")
        
        try:
            # Create a temporary container to access the volume
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
        
        # If we still don't have the notebook, return error with helpful message
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
    
    Expected JSON payload:
    {
        "notebookPath": "uploads/notebooks/uuid_filename.ipynb",
        "notebookFilename": "original_filename.ipynb"
    }
    """
    import docker
    
    logger.info(f"Preparing notebook for user {student_id}, assignment {assignment_name}")
    
    data = request.get_json() or {}
    notebook_path = data.get('notebookPath')  # Path in uploads
    notebook_filename = data.get('notebookFilename')  # Clean filename for user
    
    if not notebook_path or not notebook_filename:
        return jsonify({
            'success': False,
            'error': 'notebookPath and notebookFilename are required'
        }), 400
    
    try:
        client = docker.from_env()
        
        # Find the user's container
        logger.info(f"Looking for container with JUPYTERHUB_USER={student_id}")
        
        container = None
        containers = client.containers.list(filters={'name': 'ujaiclub'})
        for c in containers:
            env_vars = c.attrs.get('Config', {}).get('Env', [])
            for env in env_vars:
                if env == f'JUPYTERHUB_USER={student_id}':
                    container = c
                    logger.info(f"Found container {c.name} for user {student_id}")
                    break
            if container:
                break
        
        if not container:
            return jsonify({
                'success': False,
                'error': f'No running container found for user {student_id}. Please start your JupyterHub session first.'
            }), 404
        
        # Read the source notebook from the shared volume
        # The notebook path is relative to uploads, mounted at /srv/notebooks
        source_path = f"/srv/notebooks/{notebook_path.replace('uploads/', '')}"
        
        # Read from the grading service's mounted volume
        if not os.path.exists(source_path):
            # Try alternate path
            source_path = f"/srv/notebooks/notebooks/{os.path.basename(notebook_path)}"
        
        if not os.path.exists(source_path):
            return jsonify({
                'success': False,
                'error': f'Source notebook not found at {source_path}'
            }), 404
        
        # Read and process the notebook (remove solutions for students)
        import json as json_module
        
        with open(source_path, 'r', encoding='utf-8') as f:
            notebook = json_module.load(f)
        
        # Process notebook inline to create student version
        def remove_solution_code(source):
            lines = source.split('\n')
            result = []
            in_solution = False
            indent = ""
            for line in lines:
                if '### BEGIN SOLUTION' in line or '# BEGIN SOLUTION' in line:
                    in_solution = True
                    indent = line[:len(line) - len(line.lstrip())]
                    result.append(indent + '# YOUR CODE HERE')
                    result.append(indent + 'raise NotImplementedError()')
                    continue
                elif '### END SOLUTION' in line or '# END SOLUTION' in line:
                    in_solution = False
                    continue
                if not in_solution:
                    result.append(line)
            return '\n'.join(result)
        
        def remove_hidden_tests(source):
            lines = source.split('\n')
            result = []
            in_hidden = False
            for line in lines:
                if '### BEGIN HIDDEN TESTS' in line or '# BEGIN HIDDEN TESTS' in line:
                    in_hidden = True
                    continue
                elif '### END HIDDEN TESTS' in line or '# END HIDDEN TESTS' in line:
                    in_hidden = False
                    continue
                if not in_hidden:
                    result.append(line)
            return '\n'.join(result)
        
        # Process each cell
        for cell in notebook.get('cells', []):
            if cell.get('cell_type') == 'code':
                if isinstance(cell.get('source'), list):
                    source = ''.join(cell.get('source', []))
                else:
                    source = cell.get('source', '')
                
                if '### BEGIN SOLUTION' in source or '# BEGIN SOLUTION' in source:
                    source = remove_solution_code(source)
                    cell['source'] = source
                
                if '### BEGIN HIDDEN TESTS' in source or '# BEGIN HIDDEN TESTS' in source:
                    if isinstance(cell.get('source'), list):
                        source = ''.join(cell.get('source', []))
                    else:
                        source = cell.get('source', '')
                    source = remove_hidden_tests(source)
                    cell['source'] = source
            
            # Clear outputs
            if 'outputs' in cell:
                cell['outputs'] = []
            if 'execution_count' in cell:
                cell['execution_count'] = None
        
        processed_content = json_module.dumps(notebook, indent=1)
        
        # Copy to user's container
        import tarfile
        import io
        
        # Create a tar archive with the notebook
        # Set proper ownership (jovyan user: uid=1000, gid=100)
        tar_stream = io.BytesIO()
        with tarfile.open(fileobj=tar_stream, mode='w') as tar:
            notebook_bytes = processed_content.encode('utf-8')
            tarinfo = tarfile.TarInfo(name=notebook_filename)
            tarinfo.size = len(notebook_bytes)
            tarinfo.uid = 1000  # jovyan user
            tarinfo.gid = 100   # users group
            tarinfo.mode = 0o644  # rw-r--r--
            tar.addfile(tarinfo, io.BytesIO(notebook_bytes))
        tar_stream.seek(0)
        
        # Put the file in the user's work directory
        container.put_archive('/home/jovyan/work', tar_stream)
        
        logger.info(f"Successfully copied {notebook_filename} to user {student_id}'s workspace")
        
        return jsonify({
            'success': True,
            'message': f'Notebook {notebook_filename} prepared for user {student_id}'
        })
        
    except Exception as e:
        logger.error(f"Error preparing notebook: {e}")
        return jsonify({
            'success': False,
            'error': str(e)
        }), 500


@app.route('/submit/<student_id>/<assignment_name>', methods=['POST'])
def submit_for_grading(student_id, assignment_name):
    """
    Submit a notebook for grading.
    Copies the notebook from user's container to course/submitted and triggers grading.
    """
    logger.info(f"Received submission request: student={student_id}, assignment={assignment_name}")
    
    # Get notebook filename and source path from request body
    data = request.get_json() or {}
    notebook_filename = data.get('notebookFilename', f"{assignment_name}.ipynb")
    notebook_path = data.get('notebookPath', '')  # Original source notebook path
    
    # First, ensure the source assignment is set up in nbgrader
    # This is needed for autograding to work
    source_dir = Path('/srv/nbgrader/course/source') / assignment_name
    if not source_dir.exists() and notebook_path:
        logger.info(f"Setting up source assignment {assignment_name} before grading")
        try:
            # Find the source notebook
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
    
    # Copy notebook from user container to course/submitted
    success, result = copy_notebook_from_user(student_id, assignment_name, notebook_filename)
    
    if not success:
        return jsonify({
            'success': False,
            'error': result
        }), 400
    
    # Trigger grading in background
    def grade_async():
        time.sleep(1)  # Brief delay to ensure file is written
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
    
    # Find the submission - submissions are stored in course/submitted, not exchange
    submission_dir = Path('/srv/nbgrader/course') / 'submitted' / student_id / assignment_name
    
    if not submission_dir.exists():
        return jsonify({
            'success': False,
            'error': 'Submission not found'
        }), 404
    
    # Find notebook file
    notebooks = list(submission_dir.glob('*.ipynb'))
    if not notebooks:
        return jsonify({
            'success': False,
            'error': 'No notebook found in submission'
        }), 404
    
    # Grade in background
    def grade_async():
        submission_handler.process_submission(notebooks[0])
    
    threading.Thread(target=grade_async, daemon=True).start()
    
    return jsonify({
        'success': True,
        'message': 'Grading started'
    })


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
    
    # Verify source notebook exists
    if not os.path.exists(source_notebook_path):
        raise FileNotFoundError(f"Source notebook not found: {source_notebook_path}")
    
    # Create directory structure
    source_dir = Path(course_dir) / 'source' / assignment_name
    release_dir = Path(course_dir) / 'release' / assignment_name
    
    source_dir.mkdir(parents=True, exist_ok=True)
    release_dir.mkdir(parents=True, exist_ok=True)
    
    # Get notebook filename (remove UUID prefix if present)
    original_filename = Path(source_notebook_path).name
    # Remove UUID prefix: pattern like "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx_"
    notebook_name = re.sub(r'^[a-f0-9-]{36}_', '', original_filename)
    
    logger.info(f"Notebook name (cleaned): {notebook_name}")
    
    # Read and process the source notebook to add nbgrader metadata
    source_dest = source_dir / notebook_name
    add_nbgrader_metadata(source_notebook_path, str(source_dest))
    logger.info(f"Copied source notebook with nbgrader metadata to: {source_dest}")
    
    # Run nbgrader generate_assignment to register the assignment and create release version
    try:
        cmd = [
            'nbgrader', 'generate_assignment', assignment_name,
            '--force',
            '--IncludeHeaderFooter.header=',  # Empty string disables header
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
            # Try fallback method
            logger.info("Falling back to manual release creation")
        else:
            logger.info(f"Successfully generated assignment {assignment_name}")
    except subprocess.TimeoutExpired:
        logger.warning(f"nbgrader generate_assignment timed out for {assignment_name}")
    except Exception as e:
        logger.warning(f"Failed to run nbgrader generate_assignment: {e}")
    
    # Also create a fallback release directory manually if nbgrader didn't create it
    release_dest = release_dir / notebook_name
    if not release_dest.exists():
        try:
            # Make sure the release directory exists
            release_dir.mkdir(parents=True, exist_ok=True)
            process_notebook_for_students(str(source_dest), str(release_dest))
            logger.info(f"Created student version at: {release_dest}")
        except Exception as e:
            logger.error(f"Failed to create student version: {e}")
            # Last resort - just copy the source
            import shutil
            shutil.copy2(str(source_dest), str(release_dest))
            logger.info(f"Copied source as fallback to: {release_dest}")
    
    # Verify the release file exists
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
        # Get cell source as string
        if isinstance(cell.get('source'), list):
            source = ''.join(cell.get('source', []))
        else:
            source = cell.get('source', '')
        
        # Initialize metadata if needed
        if 'metadata' not in cell:
            cell['metadata'] = {}
        
        # Check for solution markers
        is_solution = ('### BEGIN SOLUTION' in source or '# BEGIN SOLUTION' in source)
        # Check for hidden test markers  
        is_test = ('### BEGIN HIDDEN TESTS' in source or '# BEGIN HIDDEN TESTS' in source)
        # Check for visible assertions (simple test cells)
        has_assert = 'assert ' in source and not is_test
        
        if is_solution or is_test or has_assert:
            cell_counter += 1
            grade_id = f"cell_{cell_counter}"
            
            # Calculate points - default 10 per graded cell
            points = 10
            total_points += points
            
            nbgrader_meta = {
                "grade_id": grade_id,
                "locked": False,
                "schema_version": 3,
            }
            
            if is_solution:
                # Solution cell - student writes code here
                # Do NOT include "points" field for solution cells
                nbgrader_meta["solution"] = True
                nbgrader_meta["grade"] = False
                nbgrader_meta["task"] = False
                logger.info(f"Marked cell {cell_counter} as solution cell")
            elif is_test or has_assert:
                # Test cell - contains assertions
                nbgrader_meta["solution"] = False
                nbgrader_meta["grade"] = True
                nbgrader_meta["points"] = points
                nbgrader_meta["task"] = False
                nbgrader_meta["locked"] = True  # Lock test cells
                logger.info(f"Marked cell {cell_counter} as test cell with {points} points")
            
            cell['metadata']['nbgrader'] = nbgrader_meta
    
    logger.info(f"Processed notebook with {cell_counter} graded cells, total {total_points} points")
    
    # Write the processed notebook
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(notebook, f, indent=1)


def process_notebook_for_students(source_path, output_path):
    """
    Process a source notebook to create a student version.
    Removes solution code and hidden tests.
    """
    import json
    
    def remove_solution_code(source):
        """Remove code between ### BEGIN SOLUTION and ### END SOLUTION markers."""
        lines = source.split('\n')
        result = []
        in_solution = False
        indent = ""
        
        for line in lines:
            if '### BEGIN SOLUTION' in line or '# BEGIN SOLUTION' in line:
                in_solution = True
                # Detect indentation
                indent = line[:len(line) - len(line.lstrip())]
                result.append(indent + '# YOUR CODE HERE')
                result.append(indent + 'raise NotImplementedError()')
                continue
            elif '### END SOLUTION' in line or '# END SOLUTION' in line:
                in_solution = False
                continue
            
            if not in_solution:
                result.append(line)
        
        return '\n'.join(result)
    
    def remove_hidden_tests(source):
        """Remove code between ### BEGIN HIDDEN TESTS and ### END HIDDEN TESTS markers."""
        lines = source.split('\n')
        result = []
        in_hidden = False
        
        for line in lines:
            if '### BEGIN HIDDEN TESTS' in line or '# BEGIN HIDDEN TESTS' in line:
                in_hidden = True
                continue
            elif '### END HIDDEN TESTS' in line or '# END HIDDEN TESTS' in line:
                in_hidden = False
                continue
            
            if not in_hidden:
                result.append(line)
        
        return '\n'.join(result)
    
    try:
        with open(source_path, 'r', encoding='utf-8') as f:
            notebook = json.load(f)
    except Exception as e:
        logger.error(f"Error reading notebook: {e}")
        # Just copy as-is if we can't parse
        import shutil
        shutil.copy2(source_path, output_path)
        return
    
    # Process each cell
    for cell in notebook.get('cells', []):
        if cell.get('cell_type') == 'code':
            # Handle both string and list source formats
            if isinstance(cell.get('source'), list):
                source = ''.join(cell.get('source', []))
            else:
                source = cell.get('source', '')
            
            # Check if this is a solution cell
            if '### BEGIN SOLUTION' in source or '# BEGIN SOLUTION' in source:
                processed = remove_solution_code(source)
                cell['source'] = processed
            
            # Check if this has hidden tests
            if '### BEGIN HIDDEN TESTS' in source or '# BEGIN HIDDEN TESTS' in source:
                # Re-read source in case it was modified
                if isinstance(cell.get('source'), list):
                    source = ''.join(cell.get('source', []))
                else:
                    source = cell.get('source', '')
                processed = remove_hidden_tests(source)
                cell['source'] = processed
        
        # Clear outputs
        if 'outputs' in cell:
            cell['outputs'] = []
        if 'execution_count' in cell:
            cell['execution_count'] = None
    
    # Write the processed notebook
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(notebook, f, indent=1)
    
    logger.info(f"Processed notebook saved to: {output_path}")


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
    
    # Start file watcher in background thread
    watcher_thread = threading.Thread(target=run_watcher, daemon=True)
    watcher_thread.start()
    
    # Run Flask API
    port = int(os.environ.get('GRADING_SERVICE_PORT', 9100))
    logger.info(f"Starting HTTP API on port {port}")
    app.run(host='0.0.0.0', port=port, threaded=True)


if __name__ == '__main__':
    main()
