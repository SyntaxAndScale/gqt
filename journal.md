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
- **Versioning:** Incremented version to `0.1.1`.

## 2026-05-01 (Persistence Phase 1)
- **Persistence:** Implemented Phase 1 of the Persistence & Sync architecture.
    - Added `rusqlite` (with bundled features) and `directories` crates.
    - Implemented XDG-compliant path resolution for the SQLite database (`$XDG_DATA_HOME/gqt/gqt.db`).
    - Created `src/db.rs` to handle schema initialization for `queues`, `tasks`, and `transactions`.
    - Integrated `Database` into `App` state using `Arc<Mutex<Database>>` to support future background sync.
- **Versioning:** Incremented version to `0.1.2`.

