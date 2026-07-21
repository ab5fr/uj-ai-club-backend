# UJ AI Club — Backend

The REST API powering the **University of Jordan AI Club** platform. Built with **Rust** and **Axum**, it handles authentication (Firebase), content management, weekly challenge workflows, notebook grading via JupyterHub/nbgrader, and leaderboards — all backed by **PostgreSQL** (Neon in production).

**Production API:** [api.uj-aiclub.com](https://api.uj-aiclub.com)

---

## Table of Contents

- [Overview](#overview)
- [Tech Stack](#tech-stack)
- [Architecture](#architecture)
- [Project Structure](#project-structure)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Environment Variables](#environment-variables)
- [Database Setup](#database-setup)
- [Running Locally](#running-locally)
- [Running with Docker](#running-with-docker)
- [Building for Production](#building-for-production)
- [API Reference](#api-reference)
- [Authentication](#authentication)
- [Challenge & Grading Pipeline](#challenge--grading-pipeline)
- [File Uploads](#file-uploads)
- [Deployment](#deployment)
- [Testing](#testing)
- [Troubleshooting](#troubleshooting)
- [Related Repos](#related-repos)

---

## Overview

This backend is a single **Axum** binary that exposes a JSON REST API on port **8000**. It serves the [Next.js frontend](../uj-ai-club-frontend/README.md) and integrates with:

- **Firebase Auth** — verifies client ID tokens via Google JWKS
- **PostgreSQL** — user data, challenges, submissions, leaderboards, articles, contact
- **JupyterHub** — interactive notebook environment for challenge attempts
- **Grading service** — nbgrader-based automated grading with webhook callbacks

Migrations run automatically at startup via `sqlx::migrate!()`.

---

## Tech Stack

| Layer            | Technology                                                             |
| ---------------- | ---------------------------------------------------------------------- |
| Language         | Rust (edition 2024)                                                    |
| Web framework    | [Axum](https://github.com/tokio-rs/axum) 0.8                           |
| Async runtime    | Tokio                                                                  |
| Database         | PostgreSQL via [sqlx](https://github.com/launchbadge/sqlx)             |
| Auth             | Firebase ID tokens (JWKS verification) + custom JWT for JupyterHub SSO |
| HTTP middleware  | tower-http (CORS, static file serving)                                 |
| Serialization    | serde / serde_json                                                     |
| Logging          | tracing + tracing-subscriber                                           |
| Containerization | Docker, docker-compose                                                 |
| Reverse proxy    | (not in this repo; configure at the host)                              |
| CI/CD            | GitHub Actions → GHCR                                                  |

---

## Architecture

```mermaid
flowchart TB
  client[Next.js Frontend] -->|"Bearer Firebase token"| api[Axum API :8000]
  api --> db[(PostgreSQL / Neon)]
  api -->|"Verify JWT"| firebase[Firebase JWKS]
  api --> uploads["/uploads static files"]
  api -->|"Start challenge"| jupyter[JupyterHub :8888]
  api -->|"Submit notebook"| grading[Grading Service :9100]
  grading -->|"POST /webhooks/nbgrader/grade"| api
  jupyter -->|"SSO JWT"| api
```

**Request flow:**

```text
HTTP Request
  → CORS layer
  → Axum router (routes/mod.rs)
  → Auth extractor (AuthUser / AdminUser) — verifies Firebase token
  → Handler — business logic + sqlx queries
  → JSON response
```

Static uploads are served at `/uploads/*` from the `uploads/` directory on disk.

---

## Project Structure

```text
uj-ai-club-backend/
├── src/
│   ├── main.rs                   # Entry point: DB connect, migrations, server bind
│   ├── lib.rs                    # create_app(), AppState, CORS, static files
│   ├── auth.rs                   # AuthUser / AdminUser extractors, JupyterHub JWT
│   ├── firebase.rs               # Firebase ID token verification (JWKS cache)
│   ├── error.rs                  # Unified error types and HTTP responses
│   ├── models.rs                 # Shared data models
│   ├── routes/
│   │   └── mod.rs                # All route definitions
│   └── handlers/
│       ├── auth/                 # session, complete_profile
│       ├── users/                # profile CRUD, avatar upload
│       ├── challenges/           # challenge list, start, submit, leaderboards
│       ├── leaderboards/         # global leaderboard
│       ├── articles/             # public article endpoints
│       ├── contact/              # contact form submission
│       ├── health/               # health check
│       ├── webhooks/             # nbgrader grade webhook
│       └── admin/                # admin CRUD
│           ├── articles/
│           ├── challenges/
│           ├── notebooks/
│           ├── submissions/
│           ├── contact/
│           └── upload.rs
├── migrations/                   # SQL migrations (applied at startup)
│   ├── 000_init.sql
│   └── 001_drop_resources_certificates_quotes.sql
├── jupyterhub/                   # JupyterHub + student notebook image
│   ├── README.md                 # JupyterHub/nbgrader architecture docs
│   ├── Dockerfile
│   ├── jupyterhub_config.py
│   ├── custom_authenticator.py
│   └── student-notebook/
├── grading/                      # nbgrader grading HTTP service
│   ├── Dockerfile
│   ├── grading_service.py
│   ├── notebook_processor.py
│   ├── nbgrader_config.py
│   └── process_notebook.py
├── Dockerfile                    # Multi-stage Rust build
├── docker-compose.yml            # Optional backend-only local compose
├── .env.example                  # Environment variable template
├── .github/workflows/
│   └── build-and-push-images.yml # CI: build & push to GHCR
└── Cargo.toml
```

Monorepo root also has:

| File | Purpose |
|------|---------|
| `docker-compose.dev.yml` | Full local stack: database, backend, frontend, JupyterHub, grading |
| `docker-compose.prod.yml` | Production: 3× backend + nginx LB + JupyterHub + grading + Watchtower |
| `infra/nginx/conf.d/` | Nginx TLS + upstream for `api.uj-aiclub.com` / `jupyter.aiclub-uj.com` |
---

## Prerequisites

### For local Rust development

- **Rust toolchain** — install via [rustup](https://rustup.rs/) (stable channel)
- **PostgreSQL** — local instance or a [Neon](https://neon.tech) database branch
- **Firebase project** — with Authentication enabled (must match frontend config)

### For full challenge/grading stack

- **Docker** and **Docker Compose**
- Docker socket access (for JupyterHub spawner and grading service)

---

## Installation

```bash
# Clone the repo
git clone <backend-repo-url>
cd uj-ai-club-backend

# Copy environment template
cp .env.example .env
```

Edit `.env` with your database URL, Firebase project ID, and JWT secret — see [Environment Variables](#environment-variables).

```bash
# Build the project
cargo build

# Run (applies migrations automatically)
cargo run
```

The server starts on `http://0.0.0.0:8000` by default.

---

## Environment Variables

### Required (Rust API)

| Variable              | Description                                                                                                 |
| --------------------- | ----------------------------------------------------------------------------------------------------------- |
| `DATABASE_URL`        | PostgreSQL connection string, e.g. `postgres://user:pass@host:5432/dbname`                                  |
| `FIREBASE_PROJECT_ID` | Firebase project ID — **required**, panics if unset. Must match frontend `NEXT_PUBLIC_FIREBASE_PROJECT_ID`. |
| `JWT_SECRET`          | Secret for signing JupyterHub SSO tokens — **required**, panics if unset                                    |

> **Note:** `FIREBASE_PROJECT_ID` is required at runtime but is **not yet listed in `.env.example`**. Add it manually.

### Optional (Rust API)

| Variable                  | Default                         | Description                                               |
| ------------------------- | ------------------------------- | --------------------------------------------------------- |
| `SERVER_ADDRESS`          | `0.0.0.0:8000`                  | Bind address and port                                     |
| `POSTGRES_USER`           | `uj_ai_club`                    | Used if `DATABASE_URL` is not set                         |
| `POSTGRES_PASSWORD`       | —                               | Required if `DATABASE_URL` is not set                     |
| `POSTGRES_DB`             | `uj_ai_club`                    | Used if `DATABASE_URL` is not set                         |
| `POSTGRES_HOST`           | `database`                      | Used if `DATABASE_URL` is not set                         |
| `GRADING_SERVICE_URL`     | `http://localhost:9100`         | Grading service endpoint                                  |
| `JUPYTERHUB_URL`          | `https://jupyter.aiclub-uj.com` | JupyterHub URL for challenge starts                       |
| `NBGRADER_WEBHOOK_SECRET` | (empty)                         | Webhook auth secret; empty = skip verification (dev only) |
| `RUST_LOG`                | —                               | Log level filter, e.g. `info` or `debug`                  |

### Alternative: compose DATABASE_URL from parts

If `DATABASE_URL` is not set, the server builds it from `POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DB`, and `POSTGRES_HOST`.

### JupyterHub & grading (Docker Compose)

These are used by `docker-compose.yml` and the JupyterHub/grading services — not by the Rust binary directly:

| Variable                 | Description                                                                           |
| ------------------------ | ------------------------------------------------------------------------------------- |
| `JUPYTERHUB_ADMIN_USERS` | Comma-separated admin emails for JupyterHub                                           |
| `JUPYTERHUB_API_TOKEN`   | JupyterHub API token                                                                  |
| `GRADING_WEBHOOK_URL`    | URL grading service calls back to, e.g. `http://backend:8000/webhooks/nbgrader/grade` |
| `MAIN_APP_URL`           | Frontend URL for JupyterHub authenticator                                             |
| `ADMIN_USERS_API`        | API endpoint for admin user lookup                                                    |
| `EXCHANGE_ROOT`          | nbgrader exchange directory                                                           |
| `COURSE_ID`              | nbgrader course identifier                                                            |
| `DOCKER_SOCKET`          | Docker socket path                                                                    |
| `GRADING_SERVICE_PORT`   | Grading service port (default `9100`)                                                 |
| `NOTEBOOKS_VOLUME_NAME`  | Docker volume for notebook storage                                                    |
| `NOTEBOOKS_HOST_PATH`    | Host path for notebook volume                                                         |
| `DOCKER_NETWORK_NAME`    | Docker network name                                                                   |

### Legacy (unused by Rust code)

These appear in `.env.example` from a previous Google OAuth implementation and are **not used** by the current Firebase-based auth:

- `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, `GOOGLE_REDIRECT_URI`
- `FRONTEND_URL`

---

## Database Setup

### Neon (recommended for production)

1. Create a project at [neon.tech](https://neon.tech).
2. Copy the connection string into `DATABASE_URL`.
3. Start the server — migrations apply automatically.

### Local PostgreSQL

```bash
# Create database
createdb uj_ai_club

# Set in .env
DATABASE_URL=postgres://your_user:your_pass@localhost:5432/uj_ai_club
```

### Migrations

Migrations in `migrations/` are applied automatically on startup:

| File                                           | Purpose                                                                                          |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `000_init.sql`                                 | Core schema: users, challenges, leaderboards, articles, contact, notebooks, submissions          |
| `001_drop_resources_certificates_quotes.sql`   | Drops legacy `resources`, `certificates`, and `quotes` tables (for DBs that already had them)    |

**Key tables:** `users`, `challenges`, `challenge_notebooks`, `challenge_submissions`, `leaderboards`, `leaderboard_entries`, `challenge_leaderboard`, `user_stats`, `contact_messages`, `articles`

To run migrations manually (without starting the server):

```bash
cargo install sqlx-cli
sqlx migrate run --database-url "$DATABASE_URL"
```

---

## Running Locally

### API only (simplest)

```bash
# Ensure .env is configured with DATABASE_URL, FIREBASE_PROJECT_ID, JWT_SECRET
cargo run
```

Verify: `curl http://localhost:8000/health`

Then start the [frontend](../uj-ai-club-frontend/README.md) with `NEXT_PUBLIC_API_URL=http://localhost:8000` and matching Firebase config.

### Full stack (recommended — monorepo root)

```bash
# From uj-ai-club/ (repo root)
docker compose -f docker-compose.dev.yml up --build
```

This starts database, backend, frontend, JupyterHub, and grading.

### Backend-only (this package)

```bash
docker compose up -d --build database jupyterhub grading
```

Then run the API on the host with `cargo run` (or use `--profile docker-backend`).

| Service    | Port | Description              |
| ---------- | ---- | ------------------------ |
| Frontend   | 3000 | Next.js (root compose)   |
| Database   | 5432 | Local PostgreSQL         |
| Backend    | 8000 | Rust Axum server         |
| JupyterHub | 8888 | Notebook environment     |
| Grading    | 9100 | nbgrader grading service |

Point the frontend's `NEXT_PUBLIC_API_URL` to `http://localhost:8000`.

---

## Running with Docker

```bash
# Full local stack (repo root)
docker compose -f docker-compose.dev.yml up --build

# Backend package only — infra
docker compose up -d --build database jupyterhub grading

# Backend package — also run the API container
docker compose --profile docker-backend up -d --build

# Production (repo root; GHCR images + nginx LB)
docker compose -f docker-compose.prod.yml up -d
```

---

## Building for Production

```bash
# Release binary
cargo build --release

# Binary location
./target/release/uj-ai-club-backend
```

### Docker image

```bash
docker build -t uj-ai-club-backend .
docker run -p 8000:8000 --env-file .env uj-ai-club-backend
```

The `Dockerfile` is a multi-stage production build (`rust:1.92-bookworm` → `debian:bookworm-slim`), runs as a non-root user, caches Cargo artifacts via BuildKit, and exposes a `/health` healthcheck on port 8000.

---

## API Reference

All routes are defined in `src/routes/mod.rs`. Base URL: `http://localhost:8000` (dev) or `https://api.uj-aiclub.com` (prod).

### Health

| Method | Route     | Auth | Description  |
| ------ | --------- | ---- | ------------ |
| GET    | `/health` | None | Health check |

### Auth

| Method | Route                    | Auth            | Description                            |
| ------ | ------------------------ | --------------- | -------------------------------------- |
| POST   | `/auth/session`          | Firebase Bearer | Sync session, auto-create Google users |
| POST   | `/auth/complete-profile` | AuthUser        | Complete profile after first sign-in   |

### Public content

| Method | Route                | Auth | Description                           |
| ------ | -------------------- | ---- | ------------------------------------- |
| GET    | `/leaderboards`      | None | Global leaderboard                    |
| GET    | `/articles`          | None | List articles                         |
| GET    | `/articles/{slug}`   | None | Article detail                        |
| POST   | `/contact`           | None | Submit contact message (rate-limited) |

### Challenges (authenticated)

| Method | Route                          | Auth     | Description                          |
| ------ | ------------------------------ | -------- | ------------------------------------ |
| GET    | `/challenges`                  | AuthUser | List challenges with notebooks       |
| GET    | `/challenges/current`          | AuthUser | Current active challenge             |
| GET    | `/challenges/leaderboard`      | AuthUser | Challenge leaderboard                |
| GET    | `/challenges/{id}/leaderboard` | AuthUser | Per-challenge submission leaderboard |
| GET    | `/challenges/{id}/submission`  | AuthUser | User's submission for a challenge    |
| POST   | `/challenges/{id}/start`       | AuthUser | Start challenge (opens JupyterHub)   |
| POST   | `/challenges/{id}/submit`      | AuthUser | Submit notebook for grading          |

### Users (authenticated)

| Method | Route            | Auth     | Description              |
| ------ | ---------------- | -------- | ------------------------ |
| GET    | `/users/profile` | AuthUser | Get current user profile |
| PUT    | `/users/profile` | AuthUser | Update profile           |
| POST   | `/users/avatar`  | AuthUser | Upload avatar image      |

### Webhooks

| Method | Route                      | Auth           | Description                           |
| ------ | -------------------------- | -------------- | ------------------------------------- |
| POST   | `/webhooks/nbgrader/grade` | Webhook secret | Receive grading results from nbgrader |

### Admin (admin role required)

| Method         | Route                                 | Description                        |
| -------------- | ------------------------------------- | ---------------------------------- |
| GET/POST       | `/admin/articles`                     | List / create articles             |
| GET/PUT/DELETE | `/admin/articles/{id}`                | Read / update / delete article     |
| PATCH          | `/admin/articles/{id}/visibility`     | Toggle visibility                  |
| GET/POST       | `/admin/challenges`                   | List / create challenges           |
| GET/PUT/DELETE | `/admin/challenges/{id}`              | Read / update / delete challenge   |
| PATCH          | `/admin/challenges/{id}/visibility`   | Toggle visibility                  |
| GET            | `/admin/challenges/{id}/notebook`     | Get notebook for challenge         |
| GET/POST       | `/admin/notebooks`                    | List / create notebooks            |
| PUT/DELETE     | `/admin/notebooks/{id}`               | Update / delete notebook           |
| POST           | `/admin/notebooks/{id}/sync`          | Sync notebook to nbgrader          |
| GET            | `/admin/submissions`                  | List all submissions               |
| GET            | `/admin/submissions/{id}/access`      | Get submission access info         |
| GET            | `/admin/submissions/{id}/file`        | Download submission file           |
| POST           | `/admin/submissions/{id}/grade`       | Manually grade submission          |
| GET            | `/admin/contact-messages`             | List contact messages              |

### Static files

| Route        | Description                                 |
| ------------ | ------------------------------------------- |
| `/uploads/*` | Uploaded files (avatars, covers, notebooks) |

---

## Authentication

### Firebase ID token verification

1. Client sends `Authorization: Bearer <firebase_id_token>`.
2. Backend fetches Google JWKS from `https://www.googleapis.com/service_accounts/v1/jwk/securetoken@system.gserviceaccount.com` (cached 1 hour).
3. Validates RS256 JWT: audience = `FIREBASE_PROJECT_ID`, issuer = `https://securetoken.google.com/{project_id}`.
4. Extracts `sub` (firebase_uid), `email`, `name`, `picture`, `sign_in_provider`.

### Session flow (`POST /auth/session`)

1. Verify Firebase token.
2. Look up user by `firebase_uid`.
3. If not found, auto-create **only for Google sign-in** (`sign_in_provider == "google.com"`).
4. Ensure `user_stats` row exists.
5. Return `{ user, needsProfileCompletion }`.

### Auth extractors

- **`AuthUser`** — any authenticated user (resolves `users.id` from `firebase_uid`).
- **`AdminUser`** — authenticated user with `users.role == "admin"`.

### JupyterHub SSO

Separate HS256 JWT signed with `JWT_SECRET`, purpose `"jupyterhub_sso"`, 1-hour expiry. Used when starting challenges and for admin notebook editing.

---

## Challenge & Grading Pipeline

```mermaid
sequenceDiagram
  participant Student
  participant API
  participant JupyterHub
  participant Grading
  participant nbgrader

  Student->>API: POST /challenges/{id}/start
  API->>API: Create JupyterHub SSO token
  API-->>Student: JupyterHub URL with token
  Student->>JupyterHub: Work on notebook
  Student->>API: POST /challenges/{id}/submit
  API->>Grading: Send notebook for grading
  Grading->>nbgrader: Run autograder
  nbgrader-->>Grading: Grade results
  Grading->>API: POST /webhooks/nbgrader/grade
  API->>API: Update submission score & leaderboard
```

See [`jupyterhub/README.md`](jupyterhub/README.md) for detailed JupyterHub/nbgrader architecture.

---

## File Uploads

Uploaded files (avatars, article covers, notebooks) are stored in the `uploads/` directory and served statically at `/uploads/*`.

In Docker, the `uploads_data` volume is shared between the API, JupyterHub, and grading services.

Admin article/notebook creation endpoints accept `multipart/form-data`.

---

## Deployment

### CI/CD

GitHub Actions workflow (`.github/workflows/build-and-push-images.yml`):

- Triggers on push to `main` (path-filtered) or manual `workflow_dispatch`
- Builds only images affected by the change (all images on manual dispatch / workflow edits)
- Images pushed to `ghcr.io/ab5fr/`:
  - `uj-ai-club-backend-server`
  - `uj-ai-club-backend-jupyterhub`
  - `uj-ai-club-backend-student-notebook`
  - `uj-ai-club-backend-grading`
- GHA layer cache, SBOM, and provenance attestations enabled
- Tags: branch name, git SHA, and `latest` on the default branch

Pull examples:

```bash
docker pull ghcr.io/ab5fr/uj-ai-club-backend-server:latest
docker pull ghcr.io/ab5fr/uj-ai-club-backend-jupyterhub:latest
docker pull ghcr.io/ab5fr/uj-ai-club-backend-student-notebook:latest
docker pull ghcr.io/ab5fr/uj-ai-club-backend-grading:latest
```

For JupyterHub, set `STUDENT_NOTEBOOK_IMAGE=ghcr.io/ab5fr/uj-ai-club-backend-student-notebook:latest` (defaults to `ujaiclub/student-notebook:latest` for local builds).

### Production

| Component  | Host                        | URL                                                    |
| ---------- | --------------------------- | ------------------------------------------------------ |
| Frontend   | Vercel                      | [uj-aiclub.com](https://uj-aiclub.com)                 |
| API        | Docker (3×) + nginx         | [api.uj-aiclub.com](https://api.uj-aiclub.com)         |
| Database   | Neon PostgreSQL             | —                                                      |
| JupyterHub | Docker                      | [jupyter.aiclub-uj.com](https://jupyter.aiclub-uj.com) |
| Auth       | Firebase                    | —                                                      |

Production stack lives at the monorepo root (`docker-compose.prod.yml`): three `backend-*` replicas behind nginx, JupyterHub, grading, certbot, and Watchtower. Images come from GHCR. The frontend is on Vercel with `NEXT_PUBLIC_API_URL=https://api.uj-aiclub.com`.

---

## Testing

No automated tests are configured yet. Dev-dependencies (`tokio-test`, `tower`, `reqwest`) are declared in `Cargo.toml` for future use.

To add tests:

```bash
cargo test
```

Integration tests would require a running PostgreSQL instance and valid Firebase credentials.

---

## Troubleshooting

| Problem                                  | Likely cause                                            | Fix                                                             |
| ---------------------------------------- | ------------------------------------------------------- | --------------------------------------------------------------- |
| Panic: `FIREBASE_PROJECT_ID must be set` | Missing env var                                         | Add `FIREBASE_PROJECT_ID` to `.env`                             |
| Panic: `JWT_SECRET`                      | Missing env var                                         | Add `JWT_SECRET` to `.env`                                      |
| Database connection failed               | Wrong `DATABASE_URL`                                    | Verify connection string; for Neon, check branch is active      |
| Migrations fail                          | Schema conflict                                         | Check migration order; for fresh DB, all 6 should apply cleanly |
| `401 Unauthorized` on all routes         | Invalid/expired Firebase token                          | Ensure frontend and backend use the same Firebase project       |
| Challenge start fails                    | JupyterHub not running                                  | Start root stack: `docker compose -f docker-compose.dev.yml up --build` |
| Grading webhook not received             | Wrong `NBGRADER_WEBHOOK_SECRET` or grading service down | Check grading service logs; verify webhook URL                  |
| CORS errors                              | Backend CORS is permissive (`Any`)                      | CORS should not block requests; check if API is reachable       |

---

## Related Repos

| Repo                                                    | Description                                 |
| ------------------------------------------------------- | ------------------------------------------- |
| [uj-ai-club-frontend](../uj-ai-club-frontend/README.md) | Next.js web application                     |
| [jupyterhub/README.md](jupyterhub/README.md)            | JupyterHub and nbgrader integration details |
