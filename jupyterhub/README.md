# JupyterHub + nbgrader Integration for UJ AI Club

This directory contains the configuration for JupyterHub with nbgrader integration for auto-grading coding challenges.

## Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Frontend      │     │   Backend       │     │   JupyterHub    │
│   (Next.js)     │────▶│   (Rust/Axum)   │────▶│   (Python)      │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                              │                        │
                              │                        │
                              ▼                        ▼
                        ┌───────────┐           ┌───────────────┐
                        │ PostgreSQL│           │ Student       │
                        │ Database  │           │ Notebooks     │
                        └───────────┘           │ (Docker)      │
                              ▲                 └───────────────┘
                              │                        │
                              │                        │
                        ┌───────────────┐              │
                        │ Grading       │◀─────────────┘
                        │ Service       │
                        └───────────────┘
```

## Components

### 1. JupyterHub (`Dockerfile`, `jupyterhub_config.py`)

- Main hub service that manages user sessions
- Uses DockerSpawner to create isolated containers for each student
- Custom JWT authenticator for SSO with main application

### 2. Custom Authenticator (`custom_authenticator.py`)

- Validates JWT tokens from the main UJ AI Club application
- Enables single sign-on (students don't need to log in again)
- Extracts username from token for JupyterHub session

### 3. Student Notebook Image (`student-notebook/`)

- Docker image based on jupyter/scipy-notebook
- Includes nbgrader for assignment submission
- Pre-configured with security restrictions

### 4. Grading Service (`grading_service.py`, `Dockerfile.grading`)

- Watches for student submissions in the nbgrader exchange
- Runs nbgrader autograde on submissions
- Reports grades back to main application via webhook

## Flow

### Student Challenge Flow

1. Student clicks "Start Challenge" on frontend
2. Backend creates a submission record and generates a JupyterHub SSO token
3. Student is redirected to JupyterHub with the token
4. JupyterHub authenticator validates token and logs student in
5. DockerSpawner creates an isolated container for the student
6. Student works on the notebook and submits via nbgrader
7. Grading service detects submission and runs autograde
8. Grading service calls backend webhook with results
9. Backend updates submission record and adds points to student's total

### Admin Notebook Setup Flow

1. Admin creates a challenge in the admin panel
2. Admin uploads a Jupyter notebook (.ipynb) with nbgrader metadata
3. Backend stores notebook and creates assignment in nbgrader
4. Notebook is distributed to students when they start the challenge

## Configuration

### Environment Variables

```bash
# Shared with main backend
JWT_SECRET=your_jwt_secret_here

# JupyterHub specific
JUPYTERHUB_URL=https://jupyter.aiclub-uj.com
JUPYTERHUB_ADMIN_USERS=admin1,admin2
JUPYTERHUB_API_TOKEN=your_api_token

# Grading webhook
NBGRADER_WEBHOOK_SECRET=your_webhook_secret
GRADING_WEBHOOK_URL=https://api.aiclub-uj.com/webhooks/nbgrader/grade
```

### Resource Limits (Per Assignment)

- **CPU Limit**: Default 0.5 cores (configurable per assignment)
- **Memory Limit**: Default 512MB (configurable: 256MB, 512MB, 1GB, 2GB)
- **Time Limit**: Default 60 minutes (configurable per assignment)
- **Network**: Disabled by default for security

## Security Features

### Network Isolation

- Student containers run on `jupyterhub-internal` network
- Network is marked as `internal: true` in Docker Compose
- No external internet access from notebooks

### Resource Limits

- CPU and memory limits prevent resource exhaustion
- Idle containers are automatically culled after 1 hour
- Maximum session age of 4 hours

### Authentication

- JWT tokens with short expiry (1 hour for JupyterHub SSO)
- Token verification on every request
- No persistent credentials stored in JupyterHub

## Building the Images

```bash
# Build JupyterHub image
docker build -t ujaiclub/jupyterhub:latest ./jupyterhub

# Build student notebook image
docker build -t ujaiclub/student-notebook:latest ./jupyterhub/student-notebook

# Build grading service image
docker build -f ./jupyterhub/Dockerfile.grading -t ujaiclub/grading-service:latest ./jupyterhub
```

## Creating nbgrader Assignments

1. Create a Jupyter notebook with problems
2. Add nbgrader metadata to cells:
   - "Autograded answer" for student code cells
   - "Autograded tests" for test cells (hidden from students)
   - "Read-only" for problem descriptions
3. Upload via admin panel with:
   - Assignment name (unique identifier)
   - Max points
   - Resource limits
4. nbgrader will automatically grade submissions

## Troubleshooting

### Student can't access JupyterHub

- Check JWT_SECRET matches between backend and JupyterHub
- Verify token hasn't expired (1 hour limit)
- Check JupyterHub logs: `docker logs uj-ai-club-jupyterhub`

### Grading not working

- Check grading service logs: `docker logs uj-ai-club-grading`
- Verify NBGRADER_WEBHOOK_SECRET matches
- Check nbgrader exchange directory permissions

### Container won't start

- Check Docker socket access
- Verify student-notebook image is available
- Check resource limits aren't too restrictive

## License

MIT License - UJ AI Club
