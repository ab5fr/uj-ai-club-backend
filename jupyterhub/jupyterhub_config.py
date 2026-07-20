"""
JupyterHub Configuration for UJ AI Club
Features:
- DockerSpawner for isolated user environments
- Custom JWT authenticator for SSO
- Resource limits (CPU, memory)
- Network isolation (no external network access)
- nbgrader integration for auto-grading
"""

import os
import sys

# Add custom authenticator to path
sys.path.insert(0, '/srv/jupyterhub')

from custom_authenticator import JWTAuthenticator

# ===========================================
# JupyterHub Core Configuration
# ===========================================

c.JupyterHub.ip = '0.0.0.0'
c.JupyterHub.port = 8000
c.JupyterHub.hub_ip = '0.0.0.0'
c.JupyterHub.hub_connect_ip = os.environ.get('JUPYTERHUB_HUB_CONNECT_IP', 'jupyterhub')

# Use the custom JWT authenticator if JWT_SECRET is set, otherwise use Dummy for dev
if os.environ.get('JWT_SECRET'):
    c.JupyterHub.authenticator_class = JWTAuthenticator
    # Only users with a valid JWT from the main API can authenticate; allow them to spawn.
    c.Authenticator.allow_all = True
else:
    # Fallback to dummy authenticator for local development
    c.JupyterHub.authenticator_class = 'jupyterhub.auth.DummyAuthenticator'
    c.DummyAuthenticator.password = 'password'

# Proxy configuration
c.ConfigurableHTTPProxy.should_start = True
c.ConfigurableHTTPProxy.api_url = 'http://127.0.0.1:8001'

# Database - use SQLite stored in /srv/jupyterhub
c.JupyterHub.db_url = 'sqlite:////srv/jupyterhub/jupyterhub.sqlite'

# Cookie secret for session management
c.JupyterHub.cookie_secret_file = '/srv/jupyterhub/jupyterhub_cookie_secret'

# ===========================================
# DockerSpawner Configuration
# ===========================================

c.JupyterHub.spawner_class = 'dockerspawner.DockerSpawner'

# Docker image for student notebooks
c.DockerSpawner.image = 'ujaiclub/student-notebook:latest'

# Use JupyterHub's single-user server entrypoint explicitly.
# The image default CMD (start-notebook.sh) does not satisfy Hub spawn health checks.
c.DockerSpawner.cmd = ['jupyterhub-singleuser']

# Remove containers when they stop
c.DockerSpawner.remove = True

# Name prefix for user containers
c.DockerSpawner.prefix = 'ujaiclub'

# Container naming
c.DockerSpawner.name_template = 'ujaiclub-{username}'

# Notebook directory inside container
c.DockerSpawner.notebook_dir = '/home/jovyan/work'

# Mount user's work directory
# For student containers to access notebooks, we need to share the same volume/bind mount
# In production: use a Docker volume name like 'uj-ai-club-backend_uploads_data'
# For local dev with backend running outside Docker: use a host path bind mount
notebooks_volume_name = os.environ.get('NOTEBOOKS_VOLUME_NAME', 'uj-ai-club-backend_uploads_data')

# Check if we should use host path (for local development on Windows/Mac)
notebooks_host_path = os.environ.get('NOTEBOOKS_HOST_PATH', '')

if notebooks_host_path:
    # Local development: use host bind mount
    c.DockerSpawner.volumes = {
        'jupyterhub-user-{username}': '/home/jovyan/work',
        'nbgrader-exchange': '/srv/nbgrader/exchange',
        notebooks_host_path: {'bind': '/srv/notebooks', 'mode': 'ro'},
    }
else:
    # Production: use named Docker volume
    c.DockerSpawner.volumes = {
        'jupyterhub-user-{username}': '/home/jovyan/work',
        'nbgrader-exchange': '/srv/nbgrader/exchange',
        notebooks_volume_name: {'bind': '/srv/notebooks', 'mode': 'ro'},
    }

# ===========================================
# Resource Limits (Default values)
# These can be overridden per-assignment
# ===========================================

# CPU limit (number of CPUs)
c.DockerSpawner.cpu_limit = 0.5

# Memory limit
c.DockerSpawner.mem_limit = '512M'

# Memory guarantee (minimum)
c.DockerSpawner.mem_guarantee = '256M'

# ===========================================
# Network Isolation
# ===========================================

# Default network for Hub <-> student (and for challenges that allow internet).
network_name = os.environ.get('DOCKER_NETWORK_NAME', 'internal')
c.DockerSpawner.network_name = network_name
c.DockerSpawner.use_internal_ip = True

# Isolated network with no outbound internet (networkDisabled challenges).
# Hub must also be attached to this network in compose.
student_isolated_network = os.environ.get(
    'STUDENT_ISOLATED_NETWORK_NAME',
    os.environ.get('DOCKER_NETWORK_NAME', 'internal'),
)

# Don't use extra_host_config network_mode as it conflicts with network_name
c.DockerSpawner.extra_host_config = {}

# Environment variables for containers
c.DockerSpawner.environment = {
    'GRANT_SUDO': 'no',
    'CHOWN_HOME': 'yes',
    'CHOWN_HOME_OPTS': '-R',
    # nbgrader configuration
    'NBGRADER_COURSE_ID': 'ujaiclub',
    # Disable network for code execution (additional security)
    'JUPYTER_ALLOW_INSECURE_WRITES': 'true',
}

# ===========================================
# Security Configuration
# ===========================================

# Don't allow named servers (one server per user)
c.JupyterHub.allow_named_servers = False

# Automatically spawn servers when users access their workspace
# This allows direct links to notebooks to work without manual spawn
c.JupyterHub.implicit_spawn_seconds = 0.5

# Timeout for spawning (seconds)
c.Spawner.start_timeout = 120
c.Spawner.http_timeout = 120

# Idle culling - shut down inactive servers
# Formgrader is admin-only via JupyterHub proxy (service admin=True).
# Do not disable XSRF or allow wildcard origins on the notebook server.
c.JupyterHub.services = [
    {
        'name': 'idle-culler',
        'command': [
            sys.executable,
            '-m', 'jupyterhub_idle_culler',
            '--timeout=3600',  # 1 hour idle timeout
            '--max-age=14400',  # 4 hour max age
        ],
    },
    {
        'name': 'formgrader',
        'url': 'http://127.0.0.1:9000',
        'command': [
            'jupyter', 'notebook',
            '--no-browser',
            '--allow-root',
            '--ip=127.0.0.1',
            '--port=9000',
            '--NotebookApp.base_url=/services/formgrader/',
            '--NotebookApp.token=',
            '--NotebookApp.password=',
            '--NotebookApp.disable_check_xsrf=False',
            '--NotebookApp.nbserver_extensions={"nbgrader.server_extensions.formgrader": true}',
        ],
        'admin': True,
    },
    # Shared by the Rust API to stop student servers after submit / session close.
    {
        'name': 'grading-service',
        'api_token': os.environ.get('JUPYTERHUB_API_TOKEN', 'default-token'),
    },
]

# RBAC roles (JupyterHub 2+). Services have no permissions unless assigned here.
# Also assign to user "grading-service" in case a legacy api_tokens mapping
# still authenticates the shared token as a user instead of the service.
c.JupyterHub.load_roles = [
    {
        'name': 'idle-culler',
        'description': 'Cull idle user servers',
        'scopes': [
            'read:users:name',
            'read:users:activity',
            'read:servers',
            'delete:servers',
        ],
        'services': ['idle-culler'],
    },
    {
        'name': 'grading-service',
        'description': 'Backend can inspect and stop student notebook servers',
        'scopes': [
            'read:users',
            'read:servers',
            'admin:servers',
            'delete:servers',
        ],
        'services': ['grading-service'],
        'users': ['grading-service'],
    },
]

# ===========================================
# Admin Configuration
# ===========================================

# Admin users who can manage the hub
admin_users_str = os.environ.get('JUPYTERHUB_ADMIN_USERS', '')
if admin_users_str:
    c.Authenticator.admin_users = set(u.strip() for u in admin_users_str.split(',') if u.strip())
else:
    c.Authenticator.admin_users = set()

# Allow admin to access user servers
c.JupyterHub.admin_access = True

# ===========================================
# nbgrader Integration
# ===========================================

# nbgrader exchange directory
c.Exchange.root = '/srv/nbgrader/exchange'

# Course ID
c.CourseDirectory.course_id = 'ujaiclub'

# ===========================================
# Webhook Configuration
# ===========================================

# API token is configured on the grading-service entry in c.JupyterHub.services.
# (c.JupyterHub.api_tokens is deprecated and does not grant RBAC scopes.)

# ===========================================
# Logging
# ===========================================

c.JupyterHub.log_level = 'INFO'
c.Spawner.debug = False

# ===========================================
# Custom Spawner Hooks
# ===========================================

SPAWN_CONFIG_FILENAME = '.ujaiclub_spawn.json'


def _docker_escape(name):
    """Match dockerspawner's escape() for volume names ('_' -> '-5f')."""
    safe = set('abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-')
    escape_char = '-'
    out = []
    for c in name:
        if c in safe:
            out.append(c)
        else:
            out.append(escape_char)
            out.append(f'{ord(c):02x}')
    return ''.join(out)


def _read_spawn_config_from_volume(username):
    """Read per-challenge spawn settings staged onto the user's work volume."""
    import json
    import docker

    volume_name = f'jupyterhub-user-{_docker_escape(username)}'
    try:
        client = docker.from_env()
        output = client.containers.run(
            'alpine:latest',
            ['cat', f'/data/{SPAWN_CONFIG_FILENAME}'],
            volumes={volume_name: {'bind': '/data', 'mode': 'ro'}},
            remove=True,
            detach=False,
        )
        if isinstance(output, bytes):
            output = output.decode('utf-8')
        return json.loads(output)
    except Exception:
        return None


def _validate_spawn_with_backend(username):
    """Ask the API whether this JupyterHub user may start a server right now."""
    import json
    import urllib.error
    import urllib.request

    backend_url = os.environ.get('BACKEND_URL', 'http://api:8000').rstrip('/')
    secret = os.environ.get('GRADING_SERVICE_SECRET', '')
    if not secret:
        return {'allowed': False, 'message': 'GRADING_SERVICE_SECRET not configured'}

    req = urllib.request.Request(
        f'{backend_url}/internal/jupyterhub/validate-spawn',
        data=json.dumps({'username': username}).encode('utf-8'),
        headers={
            'Content-Type': 'application/json',
            'X-Grading-Service-Secret': secret,
        },
        method='POST',
    )
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            return json.loads(resp.read().decode('utf-8'))
    except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError, json.JSONDecodeError) as e:
        return {'allowed': False, 'message': f'Spawn validation failed: {e}'}


def pre_spawn_hook(spawner):
    """
    Hook called before spawning a user's server.
    Blocks student spawns after challenge time expires (hub cookie alone is not enough).
    Applies challenge resource/network settings from prepare-notebook.
    """
    username = spawner.user.name
    spawner.log.info(f"Pre-spawn hook for user: {username}")

    # Students must have an unexpired in-progress session. Admins are always allowed.
    if username.startswith('user_'):
        validation = _validate_spawn_with_backend(username)
        if not validation.get('allowed'):
            message = validation.get('message') or 'Challenge session expired'
            spawner.log.warning(f"Spawn denied for {username}: {message}")
            raise Exception(message)

    cfg = _read_spawn_config_from_volume(username) or {}
    network_disabled = cfg.get('networkDisabled', True)
    if network_disabled:
        spawner.network_name = student_isolated_network
        spawner.log.info(
            f"Network disabled for {username}; using isolated network {student_isolated_network}"
        )
    else:
        spawner.network_name = network_name
        spawner.log.info(
            f"Network enabled for {username}; using network {network_name}"
        )

    cpu_limit = cfg.get('cpuLimit')
    if cpu_limit is not None:
        try:
            spawner.cpu_limit = float(cpu_limit)
        except (TypeError, ValueError):
            pass

    memory_limit = cfg.get('memoryLimit')
    if memory_limit:
        spawner.mem_limit = str(memory_limit)
    
    # IMPORTANT:
    # Overriding the single-user command can break JupyterHub health checks and OAuth flow.
    # Keep legacy behavior opt-in only; default is to use DockerSpawner's standard
    # jupyterhub-singleuser entrypoint.
    use_legacy_prespawn_cmd = os.environ.get('JUPYTERHUB_USE_LEGACY_PRESPAWN_CMD', 'false').lower() == 'true'

    if use_legacy_prespawn_cmd:
        spawner.log.warning('JUPYTERHUB_USE_LEGACY_PRESPAWN_CMD=true: using legacy custom start command')
        # Use a custom command that copies and processes notebooks before starting Jupyter
        # The uploads volume is mounted at /srv/notebooks, and notebooks are in the 'notebooks' subdirectory
        spawner.cmd = [
        '/bin/bash', '-c',
        '''
        # Python script to process notebooks (remove solutions and hidden tests)
        python3 << 'PYTHON_SCRIPT'
import json
import os
import sys

def remove_solution_code(source):
    """Remove code between ### BEGIN SOLUTION and ### END SOLUTION markers."""
    lines = source.split('\\n')
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
    
    return '\\n'.join(result)

def remove_hidden_tests(source):
    """Remove code between ### BEGIN HIDDEN TESTS and ### END HIDDEN TESTS markers."""
    lines = source.split('\\n')
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
    
    return '\\n'.join(result)

def process_notebook(source_path, output_path):
    """Process a source notebook to create a student version."""
    try:
        with open(source_path, 'r', encoding='utf-8') as f:
            notebook = json.load(f)
    except Exception as e:
        print(f"Error reading notebook: {e}")
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
    
    print(f"Processed notebook saved to: {output_path}")

# Main processing
notebooks_dir = "/srv/notebooks/notebooks"
work_dir = "/home/jovyan/work"

print(f"Looking for notebooks in {notebooks_dir}...")

if os.path.isdir(notebooks_dir):
    for filename in os.listdir(notebooks_dir):
        if filename.endswith('.ipynb'):
            source_path = os.path.join(notebooks_dir, filename)
            # Remove UUID prefix from filename
            import re
            clean_name = re.sub(r'^[a-f0-9-]{36}_', '', filename)
            output_path = os.path.join(work_dir, clean_name)
            
            print(f"Found: {filename} -> {clean_name}")
            
            if not os.path.exists(output_path):
                process_notebook(source_path, output_path)
                os.chown(output_path, 1000, 100)  # jovyan:users
                print(f"Created: {clean_name}")
            else:
                print(f"Already exists: {clean_name}")
else:
    print(f"Directory not found: {notebooks_dir}")

print("\\nWorkspace contents:")
for f in os.listdir(work_dir):
    print(f"  {f}")
PYTHON_SCRIPT

        # Start Jupyter
        exec start-notebook.sh --ServerApp.token=''
        '''
        ]
    else:
        spawner.log.info('Using default jupyterhub-singleuser command for reliable spawn')

async def post_spawn_hook(spawner):
    """
    Hook called after spawning a user's server.
    """
    username = spawner.user.name
    spawner.log.info(f"Post-spawn hook for user: {username}")

c.Spawner.pre_spawn_hook = pre_spawn_hook

# ===========================================
# Grading Callback Configuration
# ===========================================

# Environment variable for the main app webhook URL
GRADING_WEBHOOK_URL = os.environ.get(
    'GRADING_WEBHOOK_URL',
    'https://api.uj-aiclub.com/webhooks/nbgrader/grade'
)

NBGRADER_WEBHOOK_SECRET = os.environ.get('NBGRADER_WEBHOOK_SECRET', '')
