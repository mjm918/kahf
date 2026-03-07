# Kahf — Claude Code Configuration

## Project Overview

Kahf is a Huly-equivalent project management platform. Rust modular monolith backend (axum, sqlx, yrs CRDT) + React 19 + Syncfusion EJ2 frontend. Event-sourced data model with PostgreSQL 17 + TimescaleDB. Self-hosted via Docker Compose.

See `.claude/docs/plan/` for full architecture documents:
- `ARCHITECTURE_DESIGN.md` — Decisions, schema, crate structure, API design
- `IMPLEMENTATION_PLAN.md` — Phases, dependencies, project structure
- `MICROSERVICES_ARCHITECTURE.md` — Huly reference architecture (what we're replacing)
- `PLUGIN_SYSTEM.md` — Huly plugin system reference
- `SERVICES_ARCHITECTURE.md` — Huly services reference
- `UI_ARCHITECTURE.md` — Huly UI layout and navigation reference

## Hard Rules — Non-Negotiable

### Architecture
- EVERY module MUST be designed as an abstraction with traits/interfaces. No tightly coupled code.
- Use dependency injection. Concrete implementations behind trait boundaries.
- Backend crates communicate through well-defined public APIs, never internal state.
- Frontend modules use hooks and stores for state — no prop drilling beyond 2 levels.

### Code Style
- NO inline comments. Ever. Zero tolerance.
- EVERY file MUST have a single `//!` block comment at the top of the file explaining what the file does AND documenting all public items within it.
- Rust: ONLY `//!` block comments at the top of the file. No `///` item-level doc comments. No `//` comments anywhere.
- TypeScript/React: ONLY `/** */` JSDoc block comments at the top of each file. No `//` comments anywhere.

### Libraries Over Custom Code
- ALWAYS prefer an established library over writing custom code.
- Check crates.io (Rust) or npm (TypeScript) FIRST before implementing anything.
- Use `context7` MCP to look up library docs when unsure about APIs.
- Syncfusion components are MANDATORY for all UI elements. No custom UI widgets when Syncfusion has an equivalent.

### Syncfusion UI — Mandatory
- ALL UI components MUST use Syncfusion EJ2 React components.
- Refer to `syncfusion-blazor-assistant` MCP for component guidance (adapt Blazor patterns to React).
- Component mapping (from ARCHITECTURE_DESIGN.md):
  - Board → KanbanComponent
  - Tasks → GridComponent, GanttComponent
  - Contacts → GridComponent
  - Calendar → ScheduleComponent
  - Chat → ChatUIComponent
  - Documents → RichTextEditorComponent + Y.js
  - Drive → FileManagerComponent
  - HR → GridComponent, DiagramComponent
  - Dashboard → DashboardLayoutComponent, ChartComponent
  - Navigation → SidebarComponent + TreeViewComponent

### Testing — Real Tests, Not Happy Path
- NEVER write tests that only verify the happy path.
- Tests MUST connect to the staging database (postgres://postgres:frf%40%21333%21%40Fg@103.209.156.107:5432/kahflane).
- Tests MUST use real data from the staging DB, not mocked data.
- Tests MUST exercise error paths, edge cases, invalid inputs, permission boundaries.
- Integration tests are the primary testing strategy — unit tests alone are insufficient.
- Use `dbhub` MCP to query the staging DB schema and data for test design.

### UI Validation — Playwright Required
- BEFORE claiming any UI feature is complete, validate it with Playwright via the `playwright` MCP.
- Navigate to the page, take a screenshot, verify the layout matches the spec.
- Test user interactions: click, fill, submit, verify state changes.
- Test error states: invalid form submissions, network errors, empty states.
- A feature is NOT complete until Playwright confirms it works visually and functionally.

### Verification Protocol
- NEVER say "done" or "complete" without running the actual code/tests.
- NEVER assume something works — prove it with execution.
- Run `cargo check` / `cargo test` for Rust changes.
- Run `bun test` / `bun run build` for frontend changes.
- Use Playwright to visually verify UI changes.

## Tech Stack Reference

### Backend (Rust)
- Web: axum 0.8, tower-http 0.6, tokio 1
- CRDT: yrs 0.21, y-sync 0.6
- DB: sqlx 0.8 (PostgreSQL 17 + TimescaleDB)
- RBAC: casbin 2 + sqlx-adapter 2
- Cache: redis 0.27
- Search: meilisearch-sdk 0.27
- Storage: aws-sdk-s3 1
- Auth: jsonwebtoken 9, argon2 0.5
- Plugins: extism 1.0
- Serialization: serde 1, serde_json 1
- Logging: tracing 0.1

### Frontend (React + TypeScript)
- React 19, React Router 7, Vite
- Syncfusion EJ2 React v27 (all components)
- Y.js 13 + y-websocket 2 + y-prosemirror 1
- Zustand 5 (state management)
- Axios 1 (HTTP client)

### Infrastructure
- PostgreSQL 17 + TimescaleDB
- Redis 7
- Meilisearch
- MinIO (S3-compatible)
- Caddy (reverse proxy + auto TLS)
- Docker Compose

## MCP Tools Usage

| Tool | When to Use |
|------|-------------|
| `dbhub` | Query staging DB schema/data for test design, verify migrations |
| `syncfusion-blazor-assistant` | Look up Syncfusion component APIs, get usage examples |
| `playwright` | Validate UI features visually, test interactions |
| `grep` | Search GitHub for implementation patterns and examples |
| `context7` | Look up library documentation (crates, npm packages) |

## Crate Structure

```
kahf/crates/
├── kahf-core/        # Domain: Event, Entity, EntityType, error types, traits
├── kahf-auth/        # Argon2 hashing, JWT issue/verify, axum auth middleware
├── kahf-rbac/        # casbin-rs integration, policy management, permission checks
├── kahf-db/          # PostgreSQL+TimescaleDB: tx_log, entities, time-travel, migrations
├── kahf-realtime/    # WebSocket hub, yrs CRDT engine, presence, notifications
├── kahf-search/      # Meilisearch: auto-index on entity changes
├── kahf-storage/     # MinIO: upload/download/delete
├── kahf-worker/      # Background: backups, cleanup, SMTP emails, indexing
├── kahf-plugin/      # Extism WASM host: plugin lifecycle, registry, host functions
├── kahf-github/      # GitHub sync: GraphQL client, webhook handler
├── kahf-ai/          # Claude API: summarize, suggest, auto-tag
├── kahf-updater/     # Auto-update: version check, download, docker compose pull, migrate
└── kahf-server/      # THE BINARY: axum router, config, app state
```

## Frontend Structure

```
frontend/src/
├── api/              # REST client, WebSocket, Y.js
├── layouts/          # AppLayout, Header, Navigator, DetailPanel
├── modules/
│   ├── board/        # Syncfusion Kanban
│   ├── tasks/        # Syncfusion DataGrid + Gantt
│   ├── contacts/     # Syncfusion DataGrid
│   ├── calendar/     # Syncfusion Scheduler
│   ├── chat/         # Syncfusion Chat UI
│   ├── documents/    # Syncfusion RTE + Y.js collab
│   ├── drive/        # Syncfusion FileManager
│   ├── hr/           # Syncfusion DataGrid + Diagram
│   └── settings/
├── hooks/            # useAuth, useWebSocket, useCollaboration
└── store/            # Zustand
```
