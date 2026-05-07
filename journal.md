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
- **Versioning:** Incremented version to `0.1.18`.

## 2026-05-02 (Customizable Keyboard Configuration)
- **Configuration:** Implemented an exhaustive, customizable keybinding system.
    - Defined a comprehensive `Action` enum representing 50+ GQueues actions.
    - Updated `config.toml` to include a `[keybindings.bindings]` section.
    - Implemented automatic provisioning: the config file is pre-loaded with GQueues web-compatible defaults on the first launch.
- **TUI:**
    - Implemented a stateful `KeyHandler` that supports multi-key chorded commands (e.g., `g` then `i`).
    - Refactored `main.rs` to process logical actions instead of raw key events.
    - Mapped `j`/`k` for navigation and `Ctrl-c` for quitting by default.
- **Versioning:** Incremented version to `0.1.19`.

## 2026-05-02 (Task Routing & Persistence Fixes)
- **API Client:** Fixed task routing issue where new tasks were defaulting to the Inbox.
    - Updated `create_task_with_idempotency` to explicitly set `parseQuickAddSyntax: false` when a `queueKey` is provided.
    - Added detailed debug logging of the outgoing JSON request body to `gqt.log`.
- **Database:** Fixed "disappearing tasks" bug by improving data integrity.
    - Refactored `upsert_task` and `add_task_local` to use NULL-safe `IS` comparisons for relationship resolution.
    - Ensured subqueries for `queue_id` and `parent_key` check both `remote_key` and `local_id`, preventing silent insert failures for local-only items.
- **Sync Engine:** Added logs to track the successful promotion of local tasks to remote GQueues tasks.
- **Versioning:** Incremented version to `0.1.20`.


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

## 2026-05-02 (Initial Sync UX & Progress Tracking)
- **Persistence Phase 5:** Implemented "Lazy Sync" for better initial user experience.
    - Added `tasks_fetched` flag to the database to track per-queue synchronization status.
    - Prioritized **Inbox** and **active queue** for immediate task retrieval.
    - Implemented sequential background fetching for remaining queues (one every 5 seconds) to avoid `429` errors during initial load.
- **UI:** Updated the status bar to show dynamic sync progress (e.g., `Syncing: 5/12 queues remaining...`).
- **Versioning:** Incremented version to `0.1.11`.

## 2026-05-02 (Hierarchical Sub-tasks)
- **UI:** Implemented hierarchical and collapsible sub-tasks in the center pane.
    - Parent tasks now show `▶` (collapsed) or `▼` (expanded) indicators.
    - Sub-tasks are indented by one space per level for visual depth.
    - Mapped the **Spacebar** to toggle task expansion in the center pane.
    - Also enabled Spacebar for category toggling in the left pane as planned.
- **DB & API:**
    - Updated `Task` model to handle recursive `subitems` from the API.
    - Added `parent_key` column to the `tasks` table for local persistence of hierarchy.
    - Refactored `upsert_task` to recursively process sub-tasks and preserve relationships.
- **Versioning:** Incremented version to `0.1.12`.

## 2026-05-02 (Enhanced Task Details)
- **UI:** Upgraded the right pane (Task Details) with rich formatting and more metadata.
    - **Tags:** Displayed on a single line with `white` text on a `yellow` background, prepended by `#`.
    - **Assignments:** Shows assignee names with a `👤` icon.
    - **Dates & Repeat:** Unified display of Creation Date, Due Date, and Repeat information on a single line with stylized magenta/blue accents.
    - **Notes:** Now has an underlined header and supports word-wrapping.
- **DB & API:**
    - Updated `Task` model and database schema to persist `tags`, `assignments`, `creation_date`, `due_date`, and `repeats`.
    - Implemented JSON serialization for complex fields in SQLite.
- **Versioning:** Incremented version to `0.1.13`.

## 2026-05-02 (HTML Stripping)
- **UI:** Implemented basic HTML cleaning for task titles and notes.
    - Added `html-escape` and `regex` dependencies.
    - Added a `clean_html` utility to `src/ui.rs` that removes HTML tags and decodes entities (e.g., `&nbsp;`).
    - Applied cleaning to the main task list and the Details pane to improve readability.
- **Versioning:** Incremented version to `0.1.14`.

## 2026-05-02 (Manual Sync Trigger)
- **Persistence & Sync:** Implemented a manual "Sync Now" trigger.
    - Added a `SyncCommand` channel to communicate between the TUI and background engine.
    - Refactored `SyncEngine` main loop using `tokio::select!` to listen for immediate commands while maintaining the periodic timer.
    - Mapped the **`s`** key in the TUI to trigger an immediate sync cycle.
- **Versioning:** Incremented version to `0.1.15`.

## 2026-05-02 (Sync Integrity & Data Refresh)
- **Persistence & Sync:** Fixed a critical bug where remote modifications were being ignored by the Sync Engine.
    - Decoupled Queue Metadata from the "Sync Point": `upsert_queue` no longer overwrites the `last_modified` timestamp on conflict.
    - New `update_queue_sync_point` method ensures the local sync timestamp only advances AFTER a successful task pull.
    - Refactored `pull_remote_changes` to process ALL modified queues in a single cycle while prioritizing the active view.
    - Implemented a "Force Refresh" for the active queue when a manual sync (`s`) is triggered.
    - Fixed `upsert_task` to correctly update the `queue_id` during conflict resolution, ensuring task moves are reflected locally.
- **Versioning:** Incremented version to `0.1.16`.

## 2026-05-02 (Scrolling Support)
- **TUI:** Implemented vertical scrolling for all three panes.
    - **Queues & Tasks:** Refactored to use Ratatui's `ListState`, enabling automatic scrolling that follows the selection.
    - **Details:** Implemented a manual scroll offset for the task detail paragraph to handle overflow notes and narrow windows.
- **State Management:** Updated `App` to manage `nav_state`, `task_state`, and `detail_scroll`.
- **UX:** `Up`/`Down` keys now handle list navigation and paragraph scrolling depending on the focused pane. Scrolling resets automatically when a new task is selected.
- **Versioning:** Incremented version to `0.1.17`.

## 2026-05-02 (TOML Configuration Migration)
- **Configuration:** Migrated from JSON to TOML for the primary configuration format.
    - Added `toml` crate dependency.
    - Updated `src/config.rs` to prioritize `$XDG_CONFIG_HOME/gqt/config.toml`.
    - Implemented automatic migration: if a legacy JSON configuration is detected, it is automatically converted to TOML and saved to the new XDG-compliant path.
- **Versioning:** Incremented version to `0.1.18`.


