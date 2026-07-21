# AGENTS.md

## Cursor Cloud specific instructions

This repo is the **Rust/Axum REST API** for the UJ AI Club platform (core product). The
JupyterHub and grading Python services are optional and only needed to test the full
notebook challenge → autograde pipeline (they require Docker socket access and a real
Firebase project, neither of which is set up here).

### Services

| Service | Command | Notes |
| --- | --- | --- |
| PostgreSQL 16 | `sudo pg_ctlcluster 16 main start` | Must be running before the API. Not auto-started on boot. |
| Rust Axum API (port 8000) | `cargo run` | Reads `.env`; auto-applies SQL migrations on startup. |
| JupyterHub / grading | `docker compose up` | Optional; needs Docker (not installed here) + real Firebase. |

### Startup caveats (non-obvious)

- **PostgreSQL is not started automatically.** Start it with `sudo pg_ctlcluster 16 main start`
  before running the API, or `/health` will report the DB unhealthy and the server exits at boot.
- The API **panics on startup if `FIREBASE_PROJECT_ID` or `JWT_SECRET` are unset.** A dev `.env`
  (git-ignored) is already present with placeholder values pointing at local Postgres
  (`DATABASE_URL=postgres://uj_ai_club:uj_ai_club_dev@localhost:5432/uj_ai_club`). If `.env` is
  missing, recreate it from `.env.example` but change `POSTGRES_HOST`/`DATABASE_URL` host from
  `postgres` to `localhost` for host-based `cargo run`.
- Local DB role/database: user `uj_ai_club` / password `uj_ai_club_dev` / db `uj_ai_club`.
- **Firebase-authenticated routes cannot be fully tested here** (no real Firebase project). Public
  routes work without auth: `GET /health`, `GET /articles`, `GET /leaderboards`, `POST /contact`.
  Protected routes correctly return `401` without a valid Firebase Bearer token.
- README references `docker-compose.dev.yml` (full stack), but that file **does not exist** in this
  repo — only `docker-compose.yml` and `docker-compose.prod.yml` are present.

### Lint / test / build / run

- Lint: `cargo clippy --all-targets` (clean). `cargo fmt --check` currently reports pre-existing
  formatting diffs — do not treat as a setup failure.
- Test: `cargo test` (no tests defined yet, exits 0).
- Build: `cargo build` (needs `pkg-config` + `libssl-dev`, already installed).
- Run: `cargo run` after Postgres is up. Verify: `curl http://localhost:8000/health`.
