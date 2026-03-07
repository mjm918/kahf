# KahfLane — Claude Code Configuration

## Project Overview

KahfLane is a Huly-equivalent project management platform. Rust modular monolith backend (axum, sqlx, yrs CRDT) + Angular 19 + Syncfusion EJ2 frontend. Azure Portal-style UI. bun as the ONLY frontend runtime/package manager. Event-sourced data model with PostgreSQL 17 + TimescaleDB. Self-hosted via Docker Compose.

See `.claude/docs/plan/` for full architecture documents:
- `ARCHITECTURE_DESIGN.md` — Decisions, schema, crate structure, API design
- `IMPLEMENTATION_PLAN.md` — Phases, dependencies, project structure
- `MICROSERVICES_ARCHITECTURE.md` — Huly reference architecture (what we're replacing)
- `PLUGIN_SYSTEM.md` — Huly plugin system reference
- `SERVICES_ARCHITECTURE.md` — Huly services reference
- `UI_ARCHITECTURE.md` — Huly UI layout and navigation reference

## Running the Project

Run `./scripts/dev.sh` to start both backend and frontend. It kills existing instances on ports 3000/4200, starts the backend (`cargo run --bin kahf`), then the frontend (`bun run start`). Ctrl+C stops both.

## Hard Rules — Non-Negotiable

### Architecture
- EVERY module MUST be designed as an abstraction with traits/interfaces. No tightly coupled code.
- Use dependency injection. Concrete implementations behind trait boundaries.
- Backend crates communicate through well-defined public APIs, never internal state.
- Frontend modules use Angular services and signals for state — no deep input drilling beyond 2 levels.
- bun is the ONLY runtime and package manager for frontend. No npm, yarn, or pnpm. Ever.

### Code Style
- NO inline comments. Ever. Zero tolerance.
- EVERY file MUST have a single `//!` block comment at the top of the file explaining what the file does AND documenting all public items within it.
- Rust: ONLY `//!` block comments at the top of the file. No `///` item-level doc comments. No `//` comments anywhere.
- TypeScript/Angular: ONLY `/** */` JSDoc block comments at the top of each file. No `//` comments anywhere.

### Libraries Over Custom Code
- ALWAYS prefer an established library over writing custom code.
- Check crates.io (Rust) or npm (TypeScript) FIRST before implementing anything.
- Use `context7` MCP to look up library docs when unsure about APIs.
- Syncfusion components are MANDATORY for all UI elements. No custom UI widgets when Syncfusion has an equivalent.

### UI Design — Azure Portal Mandatory
- The UI MUST follow the Azure Portal design language and color scheme.
- Primary color: Azure blue (#0078D4). Neutral grays: #F3F2F1, #EDEBE9, #D2D0CE. White content areas.
- Layout: Collapsible left nav blade, breadcrumb navigation, command bars, business-grade data density.
- Typography: Segoe UI font family. Flat design with subtle borders, no heavy shadows.
- DENSE styling: Use `e-small` CSS class on ALL Syncfusion components for compact, business-grade density.
- NO custom CSS styling. Ever. Use ONLY Syncfusion theme classes and Tailwind layout utilities on wrappers.
- All UI must be congested/compact — minimize whitespace, maximize information density like Azure Portal.

### UX — Defensive UI Patterns Required
- EVERY destructive action (delete, remove, leave, revoke) MUST show a Syncfusion confirmation dialog BEFORE executing.
- Use `DialogUtility.confirm()` or Syncfusion `ejs-dialog` with confirm/cancel buttons — never browser `confirm()`.
- Form submissions MUST disable the submit button and show loading state until the server responds.
- Error states MUST be shown inline using Syncfusion Message component or card-based error banners.
- Empty states MUST show a meaningful message — never leave a blank screen.
- Toast notifications (Syncfusion `ejs-toast`) for success feedback after create/update/delete operations.
- Unsaved changes MUST trigger a "discard changes?" prompt before navigation.

### Syncfusion UI — Mandatory
- ALL UI components MUST use Syncfusion EJ2 Angular components.
- ALL components MUST use `e-small` CSS class for dense/compact sizing.
- Refer to `syncfusion-angular-assistant` MCP for component guidance.
- Use Syncfusion Fluent 2 theme (`@syncfusion/ej2-fluent2-theme`) — already configured.
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

### UI Validation — Playwright Required (MANDATORY, NOT OPTIONAL)
- AFTER implementing ANY UI feature, you MUST validate it with Playwright via the `playwright` MCP. This is NOT optional.
- Start the dev server (`bun run start` in frontend/), then use Playwright to:
  1. Navigate to the page.
  2. Take a screenshot to verify layout and visual correctness.
  3. Test user interactions: click, fill forms, submit, verify state changes.
  4. Test error states: invalid form submissions, empty fields, wrong credentials.
  5. Test defensive UX: confirm dialogs on destructive actions, loading states, toast notifications.
- A feature is NOT complete until Playwright confirms it works visually and functionally.
- If Playwright reveals issues, fix them BEFORE claiming completion.

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

### Frontend (Angular + TypeScript, bun runtime)
- Angular 21, Angular Router, Angular CLI (via bun)
- Syncfusion EJ2 Angular v32 (all components, Fluent 2 theme, `e-small` dense mode)
- Y.js 13 + y-websocket 2 + y-prosemirror 1
- Angular Signals (state management — no NgRx, use signals and services)
- Axios 1 (HTTP client with JWT interceptor)
- Tailwind CSS 4 via CDN (layout utilities on wrapper elements ONLY)
- bun for ALL operations: install, run, test, build

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
| `syncfusion-angular-assistant` | Look up Syncfusion Angular component APIs, get usage examples |
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
frontend/src/app/
├── core/             # Services (auth, websocket, realtime), guards, interceptors
├── shared/           # Shared components, pipes, directives
├── layouts/          # AppLayout, Header, Navigator, DetailPanel (Azure Portal style)
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
└── store/            # NgRx or Angular Signals
```
