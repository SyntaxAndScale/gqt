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

## 2026-05-01 (Robustness Fixes)
- **API Client:** Improved robustness of response decoding.
    - Fixed `createTask` response parsing to handle top-level arrays correctly.
    - Added `#[serde(default)]` to non-essential model fields (`is_inbox`, `completed`) to prevent parsing failures when keys are omitted by the API.
    - Enhanced error messages in `GqueuesClient` to provide more context when decoding fails.
- **Versioning:** Incremented version to `0.1.5`.

## 2026-05-02 (API Optimization & Resilience)
- **API Client:** Fixed `createTask` response parsing to expect `{"results": [...]}` object wrapper.
- **Logging:** Implemented file logging using `simplelog`. All API interactions and sync events are now logged to `gqt.log` with DEBUG level detail.
- **Robustness:** Added debug logging of raw response bodies and improved error context in `GqueuesClient`.
- **Versioning:** Incremented version to `0.1.6`.

## 2026-05-02 (Sync Optimization & Rate Limiting)
- **Persistence Phase 4:** Optimized background synchronization to prevent `429 Too Many Requests`.
    - **Metadata-First Sync:** The engine now fetches queue metadata first and only pulls tasks if the `lastModified` timestamp has changed on the server.
    - **Prioritized Sync:** TUI shares the focused queue key with the Sync Engine, which ensures your active view is always reconciled first.
    - **Rate Limit Resilience:** Implemented a custom `GqueuesError` and refactored the engine to parse and respect the `Retry-After` header from API responses.
    - **Decoupled UI:** Refactored the TUI to be purely database-driven; navigating queues no longer triggers blocking API calls.
- **XDG Compliance:** Migrated configuration loading to `$XDG_CONFIG_HOME/gqt/config.json`.
- **Versioning:** Incremented version to `0.1.7`.

## 2026-05-02 (Aesthetic UI Enhancements)
- **TUI:** Replaced the large, red "Error" block with a single-line, compact status bar at the bottom.
- **Status Messaging:** Implemented descriptive emojis for status updates:
    - `✅` for successful synchronization.
    - `❌` for errors or failures.
    - `⏳` for operations in progress.
- **State Management:** Refactored `App` to manage a unified `status` string, simplifying visual feedback.
- **Versioning:** Incremented version to `0.1.8`.

## 2026-05-02 (Categorized Navigation)
- **UI:** Implemented categorized and collapsible queues in the left navigation pane.
    - Queues are now grouped under "Personal", "Team", and "Shared" headers.
    - Category headers are collapsed by default and use `▶` / `▼` indicators.
    - Queues within categories are indented by one space for visual hierarchy.
- **State Management:** Introduced `NavEntry` enum to manage the unified list of categories and queues.
- **DB & API:** Added `category` support to the database schema and API client logic.
- **Dynamic Categorization:** Refactored navigation to use the API-provided `categoryName` and `teamName`.
    - Queues are now grouped by their actual GQueues categories (e.g., "(Archive) Projects", "Shared w/ Yen").
    - Implemented priority sorting: "Personal" and "Inbox" first, followed by alphabetical, with "Archive" categories pushed to the bottom.
    - Decoupled API client from display logic, following the mandate that the consuming application handles data processing/filtering.
- **Versioning:** Incremented version to `0.1.10`.


