# Project Journal

## 2026-04-30
- Initialized project structure.
- Created GEMINI.md, prd.md, spec.md, todo.md, and journal.md.
- Initialized Rust binary project.
- Initialized Git repository and added `.gitignore`.
- Added dependencies: ratatui, crossterm, tokio, reqwest, serde, chrono, uuid, anyhow.
- Implemented basic three-pane TUI layout and navigation logic.
- Implemented core models (Queue, Task, Transaction, Operation).
- Implemented `GqueuesClient` for interacting with the Beta REST API.
- Integrated API fetching into the TUI:
    - Load configuration from `.gemini/settings.local.json`.
    - Initial fetch of queues and tasks.
    - Refresh tasks on queue selection (`Enter`).
    - Added loading and error states to UI.
    - Verified compilation with `cargo check`.
    - Created `README.md`.
    - Performed initial Git commit to `main` branch.

## 2026-05-01
- **Architectural Decision:** Plan to re-architect the code to decouple the GQueues API client into its own module, intended to be spun off as a standalone Rust library later.
- **Refactoring:** Successfully moved GQueues API client and models to a dedicated `src/gqueues` module.
    - Created `src/gqueues/models.rs`, `src/gqueues/client.rs`, and `src/gqueues/mod.rs`.
    - Updated `App` and `main.rs` to use the new module structure.
    - Verified compilation with `cargo check`.
- **Versioning:** Incremented version to `0.1.2`.

## 2026-05-01 (Persistence Phase 2)
- **Persistence:** Implemented Phase 2: Local CRUD.
    - Refactored `App` to use SQLite as the primary source of truth.
    - Implemented API-to-DB caching (upserting queues and tasks on fetch).
    - Implemented local-only task creation with placeholder `remote_key` (`local-<uuid>`).
    - Added transaction logging for local creations in the `transactions` table.
    - Updated UI to show a "Pending" indicator (`⏳`) for unsynced tasks.
    - Added `a` keybinding to create a local task (mocked title/notes for now).
- **Versioning:** Incremented version to `0.1.3`.

## 2026-05-01 (Sync Engine & Fixes)
- **Persistence Phase 3:** Implemented background Sync Engine.
    - Background loop with exponential backoff.
    - Idempotency key support in transactions.
    - Fixed `NOT NULL constraint failed: tasks.queue_id` by ensuring `queue_key` is set during pull.
    - Improved UI feedback: refresh data on both sync success and error; display sync status messages.
- **Versioning:** Incremented version to `0.1.4`.


