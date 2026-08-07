# AgriTrust

Agricultural escrow on **Soroban** (Stellar smart contracts). Farmers list
produce, buyers make offers, funds are held in escrow by a Soroban contract,
and payment is released only after delivery is confirmed. No passwords — users
authenticate with their **Freighter** wallet.

> **Status: scaffolding phase.** The monorepo structure, tooling, and CI are in
> place. No feature logic has been written yet. See
> [`docs/architecture.md`](docs/architecture.md) and
> [`docs/api-spec.md`](docs/api-spec.md) for the planned design.

## Tech stack

| Layer | Technology |
|---|---|
| Smart contract | Rust + [Soroban SDK](https://github.com/stellar/rs-soroban-sdk) (`contracts/`) |
| Backend | Django REST Framework + PostgreSQL (`backend/`) |
| Frontend | React 19 + Vite + TypeScript + Tailwind CSS (`frontend/`) |
| Wallet | [Freighter](https://www.freighter.app/) via `@stellar/freighter-api` |
| Dev tooling | Docker Compose (Postgres), pre-commit (black/ruff, eslint/prettier, cargo fmt/clippy) |
| CI | GitHub Actions — lint + format + test on push/PR |

## Folder structure

```
AgriTrust/
├── contracts/                # Soroban smart contract (Rust)
│   ├── Cargo.toml
│   └── src/lib.rs            # contract skeleton
├── backend/                  # Django REST Framework + PostgreSQL
│   ├── config/               # Django project (settings, urls, wsgi/asgi)
│   ├── tests/                # pytest smoke tests
│   ├── manage.py
│   ├── requirements.txt      # prod deps
│   ├── requirements-dev.txt  # + lint/test deps
│   └── pyproject.toml        # black/ruff/pytest config
├── frontend/                 # React + Vite + Tailwind + Freighter
│   ├── src/lib/freighter.ts  # wallet integration surface (placeholder)
│   ├── eslint.config.js
│   └── package.json
├── docs/
│   ├── architecture.md       # 3-tier design, escrow data flow, wallet auth
│   └── api-spec.md           # planned REST endpoints (draft)
├── .github/workflows/ci.yml  # lint + test on push
├── docker-compose.yml        # local PostgreSQL
└── .pre-commit-config.yaml   # shared git hooks
```

## Prerequisites

- Git
- Python 3.12+ (3.13 recommended)
- Node.js 20+ (22 recommended) and npm
- Rust toolchain (`rustup`) — for the Soroban contract
- Docker (optional, for local Postgres)
- [Freighter](https://www.freighter.app/) browser extension (frontend dev)

## Setup

### 1. Database (local Postgres)

```bash
docker compose up -d db
```

### 2. Backend

```bash
cd backend
python -m venv .venv
source .venv/bin/activate        # Windows: .venv\Scripts\activate
pip install -r requirements-dev.txt
cp .env.example .env             # adjust as needed
python manage.py migrate         # no apps yet — creates base tables
python manage.py runserver
```

Backend runs at <http://localhost:8000> (Django admin at `/admin/`).

### 3. Frontend

```bash
cd frontend
npm install
npm run dev
```

Frontend runs at <http://localhost:5173> (Vite).

### 4. Contract

```bash
cd contracts
cargo check      # compiles the scaffold contract
cargo test       # runs unit tests (none yet)
```

### 5. Pre-commit hooks

```bash
pip install pre-commit
pre-commit install
pre-commit run --all-files       # optional: check everything once
```

Hooks: **black + ruff** (Python), **eslint + prettier** (JS/TS), and
**cargo fmt + clippy** (Rust — requires `cargo` on PATH).

### 6. CI

`.github/workflows/ci.yml` runs format, lint, and tests for all three tiers on
every push to `main` and on pull requests. Backend tests use an in-memory
SQLite database, so CI needs no PostgreSQL service.

## Common commands

| Where | Command | Purpose |
|---|---|---|
| backend | `pytest` | Run tests |
| backend | `ruff check .` / `black --check .` | Lint / format check |
| frontend | `npm run build` | Typecheck + production build |
| frontend | `npm run lint` / `npm run format:check` | Lint / format check |
| contracts | `cargo clippy -- -D warnings` | Lint |
| root | `docker compose up -d db` | Start local Postgres |

## License

Proprietary — all rights reserved.
