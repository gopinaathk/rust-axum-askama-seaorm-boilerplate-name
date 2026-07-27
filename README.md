# Axum + Askama Auth Starter

A production-shaped Rust web starter: **Axum** routing, **Askama** server-rendered templates, **SeaORM** on **PostgreSQL**, **Alpine.js** for interactions, and **session based authentication** (register, sign in, sign out) with sessions stored in **Postgres or Redis**.

No JavaScript build step. No SPA. Just typed templates, hand-written CSS, and a small Alpine layer.

```
Axum 0.8 · Askama 0.16 · SeaORM 2.0 · tower-sessions 0.15 · Alpine.js 3 · Argon2id
```

---

## Contents

- [Features](#features)
- [Screens](#screens)
- [Quick start](#quick-start)
- [Configuration](#configuration)
- [Project structure](#project-structure)
- [How authentication works](#how-authentication-works)
- [Session backends](#session-backends)
- [Database and migrations](#database-and-migrations)
- [Timezones](#timezones)
- [Health checks](#health-checks)
- [Security notes](#security-notes)
- [Testing](#testing)
- [Deploying](#deploying)
- [License](#license)

---

## Features

**Authentication**
- Register with name, email and password; sign in; sign out
- Argon2id password hashing, run on a blocking thread so the runtime stays responsive
- Session id rotation on sign-in (session fixation defence)
- CSRF token on every form, compared in constant time
- Timing-equalised login: unknown emails cost the same as wrong passwords
- Guard extractors: `CurrentUser` (redirects guests) and `MaybeUser` (optional)

**Sessions**
- Server-side sessions via `tower-sessions`, backed by **Postgres** or **Redis** (`SESSION_STORE`)
- Sliding inactivity expiry, `HttpOnly` + `SameSite=Lax` cookies, `Secure` by default in production
- Expired Postgres rows swept by a background task; Redis expires keys itself
- Dashboard shows the live session: id, created time, expiry, cookie name, client IP, user agent, storage backend

**Data**
- SeaORM entities with a separate `migration` crate
- Database is created automatically on first boot when it does not exist
- Pending migrations applied on boot (`DB_RUN_MIGRATIONS`)
- Connection parts configured separately (`DB_HOST`, `DB_PORT`, `DB_USERNAME`, …), with `DATABASE_URL` as an override for managed platforms

**Web layer**
- Askama templates compiled and type-checked with your Rust code
- Flash messages that survive redirects
- Styled error pages (404 / 403 / 422 / 500) instead of bare text
- Static files served from `/static`, security headers set globally
- Graceful shutdown on Ctrl+C and SIGTERM

**Frontend, no bundler**
- Hand-written CSS with design tokens, dark and light themes (respects `prefers-color-scheme`, remembers the choice)
- Alpine.js components: theme toggle, mobile nav, password visibility, password strength meter, submit guard, copy-to-clipboard, dismissible alerts
- Accessibility basics: skip link, visible focus rings, labelled inputs, `prefers-reduced-motion`, SVG icons (no emoji)

**Operations**
- `/health` HTML status page, `/healthz` text probe, `/healthz.json` for dashboards
- `APP_ENV` profiles (`development` / `production`) that shift defaults
- Structured logs through `tracing`, filtered with `RUST_LOG`
- Every timestamp rendered in a configurable timezone (`APP_TIMEZONE`, e.g. `Asia/Kolkata`)

---

## Screens

| Route | Purpose |
| --- | --- |
| `/` | Landing page with feature overview and live user count |
| `/register` | Name, email, password with client-side strength hint |
| `/login` | Sign in, re-renders with `422` and keeps the email on failure |
| `/dashboard` | User details + current session details + sign out |
| `/health` | Database and session store status, runtime info |

---

## Quick start

**Requirements:** Rust 1.85+, PostgreSQL 14+, and Redis only if you pick the Redis session backend.

```bash
git clone https://github.com/your-user/rust-axum-askama-seaorm-boilerplate.git
cd rust-axum-askama-seaorm-boilerplate
cp .env.example .env      # Windows: copy .env.example .env
# edit .env: DB_USERNAME / DB_PASSWORD / DB_NAME
cargo run
```

Open <http://127.0.0.1:3000>.

On first boot the app creates the database if it is missing, applies migrations, and starts serving. If you prefer containers for the datastores:

```bash
docker compose up -d          # Postgres on 5432, Redis on 6379
```

> `.env` values containing spaces must be quoted, e.g. `APP_NAME="Rust Askama"`. An unquoted space makes the parser stop and everything after that line falls back to defaults.

---

## Configuration

All configuration is read once at boot from the environment. See [`.env.example`](.env.example) for the annotated list.

### Application

| Variable | Default | Notes |
| --- | --- | --- |
| `APP_ENV` | `development` | `production` turns on secure cookies by default |
| `APP_NAME` | `Rust Askama` | Shown in the navbar and page titles |
| `APP_TIMEZONE` | `UTC` | Any IANA zone, e.g. `Asia/Kolkata` |
| `HOST` / `PORT` | `127.0.0.1` / `3000` | Bind address |
| `STATIC_DIR` | `static` | Directory served at `/static` |
| `TRUST_PROXY` | `false` | Trust `X-Forwarded-For` / `X-Real-IP` (only behind your own proxy) |
| `RUST_LOG` | `rust_askama=debug,…` | `tracing` filter |

### Database

| Variable | Default | Notes |
| --- | --- | --- |
| `DB_HOST` | `localhost` | |
| `DB_PORT` | `5432` | |
| `DB_USERNAME` | `postgres` | |
| `DB_PASSWORD` | *(empty)* | Percent-encoded automatically |
| `DB_NAME` | `rust-axum-askama` | Created on boot when missing |
| `DB_ADMIN_NAME` | `postgres` | Maintenance database used for `CREATE DATABASE` |
| `DB_OPTIONS` | *(empty)* | Extra query string, e.g. `sslmode=require` |
| `DB_AUTO_CREATE` | `true` | Create the database when it does not exist |
| `DB_RUN_MIGRATIONS` | `true` | Apply pending migrations on boot |
| `DB_MAX_CONNECTIONS` / `DB_MIN_CONNECTIONS` | `10` / `1` | Pool bounds |
| `DB_CONNECT_TIMEOUT_SECS` | `8` | |
| `DATABASE_URL` | *(unset)* | Full URL; overrides every `DB_*` value above |

### Sessions and Redis

| Variable | Default | Notes |
| --- | --- | --- |
| `SESSION_STORE` | `postgres` | `postgres` or `redis` |
| `SESSION_COOKIE_NAME` | `rust_askama_sid` | |
| `SESSION_TTL_MINUTES` | `1440` | Sliding inactivity window |
| `SESSION_COOKIE_SECURE` | env dependent | `true` in production, `false` in development |
| `SESSION_CLEANUP_INTERVAL_SECS` | `600` | Postgres sweep interval |
| `REDIS_HOST` / `REDIS_PORT` | `127.0.0.1` / `6379` | |
| `REDIS_USERNAME` / `REDIS_PASSWORD` | *(empty)* | ACL user and password, percent-encoded automatically |
| `REDIS_DB` | `0` | |
| `REDIS_KEY_PREFIX` | `rust_askama:session:` | |
| `REDIS_URL` | *(unset)* | Full URL; overrides every `REDIS_*` value above |

---

## Project structure

```
.
├── migration/                     # SeaORM migration crate (own binary)
│   └── src/
│       ├── lib.rs                 # Migrator: registers migrations
│       ├── main.rs                # CLI, builds DATABASE_URL from DB_* when unset
│       ├── m20260101_000001_create_users_table.rs
│       └── m20260101_000002_create_sessions_table.rs
├── src/
│   ├── config/                    # One file per configuration section
│   │   ├── mod.rs                 # Config, Environment, timezone helpers
│   │   ├── database.rs            # DB parts -> URL, DATABASE_URL override
│   │   ├── redis.rs               # Redis parts -> URL
│   │   ├── server.rs              # Bind address, static dir
│   │   ├── session.rs             # Cookie, TTL, backend selection
│   │   └── env_vars.rs            # Typed env readers
│   ├── db/mod.rs                  # Create database, connect, migrate
│   ├── entities/                  # SeaORM models (users, sessions)
│   ├── repositories/              # Data access, the only SeaORM callers
│   ├── services/                  # Use cases: auth, validation rules
│   ├── security/password.rs       # Argon2id hash + verify
│   ├── sessions/                  # Session stores
│   │   ├── mod.rs                 # AppSessionStore: Postgres or Redis
│   │   ├── postgres.rs            # SeaORM backed store + sweeper
│   │   └── redis.rs               # Redis backed store (TTL keys)
│   ├── web/
│   │   ├── mod.rs                 # Router, static files, headers
│   │   ├── routes/                # home, auth, dashboard, health
│   │   ├── extractors.rs          # CurrentUser / MaybeUser guards
│   │   ├── session.rs             # Session keys, client info, views
│   │   ├── csrf.rs                # Token mint + constant-time verify
│   │   ├── flash.rs               # One-shot messages
│   │   └── templates.rs           # Askama template structs
│   ├── error.rs                   # AppError -> HTML error pages
│   ├── state.rs                   # AppState (db, config, store, uptime)
│   ├── lib.rs
│   └── main.rs                    # Boot: config, db, sessions, serve
├── templates/                     # layout.html + pages/
└── static/                        # css/app.css, js/app.js
```

The dependency direction is one-way: `web` → `services` → `repositories` → `entities`. Handlers never touch entities directly, which keeps the domain testable without HTTP.

---

## How authentication works

1. `GET /register` renders the form and mints a CSRF token into the session.
2. `POST /register` verifies the token, validates the input, hashes the password with Argon2id on a blocking thread, inserts the user, rotates the session id, and stores `user_id` plus sign-in metadata.
3. `GET /dashboard` resolves the session through the `CurrentUser` extractor. A session pointing at a deleted user is discarded.
4. `POST /sign-out` verifies the token, deletes the session record, and redirects with a flash message.

Failed submissions re-render the same page with `422 Unprocessable Entity`, the typed values preserved (never the password) and messages listed above the form. Login failures are deliberately vague: "Those credentials do not match our records."

---

## Session backends

```bash
SESSION_STORE=postgres   # durable, queryable, swept by a background task
SESSION_STORE=redis      # keys with native TTL, no sweeper needed
```

Both implement `tower_sessions::SessionStore` and are wrapped by `AppSessionStore`, so switching is a one-line env change. Redis connectivity is verified at boot with a `PING`, so a misconfigured cache fails fast instead of at the first request.

---

## Database and migrations

Migrations live in their own crate and can be driven from the CLI:

```bash
cargo run -p migration -- status      # what has been applied
cargo run -p migration -- up          # apply pending migrations
cargo run -p migration -- down -n 1   # roll back the last one
cargo run -p migration -- fresh       # drop everything and re-apply
```

`DATABASE_URL` is used when set; otherwise the CLI assembles it from the same `DB_*` variables the server reads, so `.env` stays the single source of truth.

To add a table: create `migration/src/mYYYYMMDD_HHMMSS_name.rs`, register it in `migration/src/lib.rs`, then mirror it with an entity in `src/entities/`.

---

## Timezones

`APP_TIMEZONE` accepts any IANA zone and every rendered timestamp uses it, abbreviation included:

```
APP_TIMEZONE=Asia/Kolkata   ->  27 Jul 2026, 19:35 IST
APP_TIMEZONE=UTC            ->  27 Jul 2026, 14:05 UTC
```

Timestamps are stored as `timestamptz` (UTC) and converted only for display, so changing the zone never rewrites data.

---

## Health checks

| Endpoint | Response |
| --- | --- |
| `/health` | HTML page: per-check status, latency, version, uptime, active sessions |
| `/healthz` | `ok` with `200`, or `unhealthy: <component>` with `503` |
| `/healthz.json` | Same checks as JSON, for dashboards and alerting |

Both probes check Postgres and the active session backend, so `503` means a dependency is actually down.

---

## Security notes

- Passwords are hashed with Argon2id (`argon2` crate defaults); the hash never leaves the repository layer, and `users::Model` is intentionally not `Serialize`.
- Session ids rotate on sign-in; sign-out deletes the stored record so a copied cookie cannot be replayed.
- Cookies are `HttpOnly` and `SameSite=Lax`, and `Secure` whenever `APP_ENV=production` (a warning is logged if you disable it there).
- Forms carry a per-session CSRF token, compared in constant time.
- `X-Forwarded-For` is ignored unless `TRUST_PROXY=true`, because any client can send it.
- Database identifiers are quoted and credentials percent-encoded when URLs are assembled.
- Responses carry `X-Content-Type-Options: nosniff` and `Referrer-Policy: strict-origin-when-cross-origin`.

Before going live: serve over TLS, set `APP_ENV=production`, keep `SESSION_COOKIE_SECURE=true`, and consider rate limiting the auth routes (this starter does not include it).

---

## Testing

```bash
cargo test           # unit tests: config, validation, CSRF, password hashing
cargo clippy --all-targets
cargo fmt --check
```

The unit tests need no database. Password hashing tests run real Argon2, so they take a moment in debug builds.

---

## Deploying

```bash
cargo build --release
```

Ship the binary together with `templates/` (compiled in, but handy to keep), `static/`, and the environment variables. A minimal production profile:

```env
APP_ENV=production
APP_TIMEZONE=Asia/Kolkata
HOST=0.0.0.0
PORT=8080
DATABASE_URL=postgres://user:password@db.internal:5432/app?sslmode=require
SESSION_STORE=redis
REDIS_URL=rediss://:password@cache.internal:6380/0
TRUST_PROXY=true
RUST_LOG=rust_askama=info,warn
```

Point your orchestrator's liveness probe at `/healthz`.

---

## License

MIT. See [LICENSE](LICENSE).

---

<sub>Keywords: rust web starter, axum boilerplate, askama templates, seaorm postgres, session authentication, argon2, alpine.js, server side rendering, tower-sessions, redis sessions.</sub>
