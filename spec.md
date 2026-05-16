# Technical Specification: Gqueues TUI (gqt)

## Overview
`gqt` is a high-performance, terminal-based task manager for GQueues. It is designed to be offline-first, using a local SQLite database and a background synchronization engine to reconcile state with the official GQueues API.

## Technology Stack
- **Language:** Rust
- **UI Framework:** Ratatui
- **Async Runtime:** Tokio
- **Persistence:** SQLite via `rusqlite`
- **Configuration:** TOML
- **API Client:** `gqueues-api-rs` (External Crate)

## Code Architecture

### 1. TUI Layer (`src/ui.rs`)
The UI is built using a three-pane layout:
- **Left (Queues):** Tree-like navigation with collapsible categories.
- **Center (Tasks):** Hierarchical task list with support for sub-tasks.
- **Right (Details):** Metadata display for the selected task.

### 2. State Management (`src/app.rs`)
A central `App` struct maintains the state of the TUI, including navigation indices (`ListState`), cached data, and UI visibility flags (e.g., help modal).

### 3. Sync Engine (`src/sync.rs`)
A dedicated background task that runs independently of the UI. It follows a dual-phase reconciliation strategy:
1. **Push Phase:** Sequentially promotes local transactions (stored in `transactions` table) to the GQueues API using idempotency keys.
2. **Pull Phase:** Fetches remote metadata first, then pulls full task data for modified or stalest queues.

### 4. Database Layer (`src/db.rs`)
SQLite acts as the single source of truth for the UI. It maintains:
- `queues`: Metadata and sync state for all folders/teams.
- `tasks`: Full task details and hierarchical relationships.
- `transactions`: An append-only log of local modifications awaiting sync.

### 5. CLI Layer (`src/commands/`)
The CLI provides a lightweight interface for rapid task entry:
- **Argument Parsing:** Uses `clap` to handle commands and flags.
- **Quick Add:** Leverages GQueues Quick Add Syntax by setting the `parseQuickAddSyntax` flag in API requests.
- **Workflow:** Saves tasks to the local database, allowing the background `SyncEngine` to handle the actual API communication asynchronously.

## API Communication
All communication with GQueues is handled by the `gqueues-api-rs` library.
- **Base Endpoint:** `https://api.gqueues.com/v0`
- **Authentication:** Bearer Token.
- **Rate Limiting:** `gqt` respects the `Retry-After` header and implements exponential backoff.

## Keyboard Controls
GQueues TUI maintains high parity with the official web client shortcuts.
- **Navigation:** `j`/`k`, `Tab`, `g`+`i` (Inbox).
- **Expansion:** `Space` for categories and sub-tasks.
- **Synchronization:** `s` for manual refresh.
- **Help:** `?` for a dynamic binding reference.

## Documentation References
- [GQueues API Documentation](https://api.gqueues.com)
- [gqueues-api-rs Repository](https://github.com/SyntaxAndScale/gqueues-api-rs)
- [GQueues Shortcuts Reference](https://www.gqueues.com/help/shortcuts)
