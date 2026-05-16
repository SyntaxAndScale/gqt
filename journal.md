# Project Journal

## 2026-05-01
- **Initial Prototype:** Implemented the three-pane TUI layout and basic GQueues API integration.
- **Library Decoupling:** Refactored the code to move the API client into a standalone module (`src/gqueues`).
- **Persistence (Phases 1 & 2):** Implemented SQLite storage, XDG-compliant path resolution, and local caching for queues and tasks.
- **Sync Engine (Phase 3):** Developed the background synchronization task with idempotency and transaction logging.
- **Robustness:** Improved API response decoding and error handling for edge cases.

## 2026-05-02
- **Sync Optimization:** Implemented metadata-first syncing and rate-limiting resilience using the `Retry-After` header.
- **Logging:** Added file-based logging (`gqt.log`) with detailed debug information for API troubleshooting.
- **Aesthetic UI:** Replaced verbose error screens with a unified status bar and descriptive emojis.

## 2026-05-03
- **Categorized Navigation:** Implemented collapsible category headers and dynamic grouping by API `categoryName` and `teamName`.
- **Initial Sync UX:** Developed "Lazy Sync" logic to prioritize the Inbox and active queue during first-run data fetching.
- **Hierarchical Sub-tasks:** Enabled recursive sub-task rendering and spacebar-toggled expansion in the center pane.
- **Enhanced Task Details:** Upgraded the right pane with rich metadata display, including tags, assignments, and formatted dates.

## 2026-05-06
- **HTML Handling:** Implemented regex-based HTML tag stripping and entity decoding for task descriptions.
- **Manual Sync:** Added the `s` shortcut to trigger an immediate background reconciliation cycle.
- **Sync Integrity:** Fixed critical bugs related to remote modification detection and task movement across queues.
- **Scrolling Support:** Integrated `ListState` and manual offsets to enable vertical scrolling across all panes.
- **TOML Migration:** Transitioned configuration from JSON to TOML with an automatic migration path.
- **Keybindings:** Implemented an exhaustive and customizable keyboard configuration system matching GQueues web defaults.

## 2026-05-07
- **Setup Wizard:** Developed a TUI-based first-run initialization flow to collect API credentials securely.
- **Bootstrap Sync:** Refined the onboarding UX to perform a high-priority sync of queues and Inbox tasks during setup.
- **Real-time Status:** Updated the Sync Engine to report granular progress (e.g., specific queue names) to the status bar.
- **Help Modal:** Implemented an interactive help screen (`?`) displaying app metadata and dynamic keybindings.

## 2026-05-08
- **Library Extraction:** Successfully externalized the GQueues API client into the standalone `gqueues-api-rs` crate, hosted on GitHub.

## 2026-05-10
- **Task Detail Redesign:** Overhauled the right pane for professional metadata display.
    - Implemented fixed-width alignment for the metadata line (Created, Repeat, Due, Assignee).
    - Added clickable GQueues web links for synced tasks.
    - Standardized date formatting: YYYY-MM-DD for creation and "Mmm d" for due dates.
    - Implemented a blockquote style for task notes (later removed for cleaner look).
    - Added placeholders for future Comments and Activity API features.
- **UI Bug Fixes & Refinements:**
    - **Date Formatting:** Fixed `creation_date` parsing to handle multiple formats and strictly show `YYYY-MM-DD`.
    - **Tag Rendering:** Removed background highlight from the space between tags for a cleaner look.
    - **Note Formatting:** Removed the `> ` prefix from task notes.
    - **Status Bar:** Reformatted "Last Synced" timestamp to fit perfectly within the status bar and use the local timezone.
    - **Task List Hierarchy:** Fixed a bug where collapsible arrows (▶/▼) were missing for tasks with subtasks when loaded from the local database.
    - **Due Date Coloring:** Implemented color-coding for the due date in the Details pane—**Green** for tasks due today or in the future, and **Red** for overdue tasks.
    - **Repeating Tasks:** Improved detection logic for the 🔁 emoji to support all recurrence formats and added a fallback for emojis present in the task title.
- **Versioning:** Incremented version to `0.1.26`.

## 2026-05-12
- **Pane Navigation Fix:** Re-implemented Shift+Tab for backward pane switching by fixing a double-prefix bug in `src/keys.rs`.
- **Inline Task Creation:** Implemented advanced task creation shortcuts with inline title editing.
    - **Shortcuts:** Added support for `i` (below), `Shift+i` (above), `o` (bottom), and `Shift+o` (top).
    - **Inline Editor:** Created a "virtual" task entry in the UI with a real-time cursor for title entry.
    - **Bulk Add:** Implemented `Tab` key behavior to save the current task and immediately start another one below.
    - **Cancellation:** Configured `Esc` to cancel and discard the new task (deliberate departure from GQueues Web UI behavior).
    - **Stable Ordering:** Updated local database queries to ensure tasks are ordered by creation date, maintaining predictable list positions.
    - **Regression Testing:** Added a unit test suite to `src/keys.rs` to verify correct key-to-string conversion for Tab, Shift+Tab (BackTab), and Control sequences.

## 2026-05-13
- **Library Model Expansion:** Updated `gqueues-api-rs` (on branch `feature/expand-models`) to support all GQueues REST API fields, including `position`, `attachments`, `section_key`, and detailed assignment metadata.
- **Task Ordering Fix:** Resolved an issue where new tasks would jump to the top of the list.
    - **Position Field:** Integrated the API `position` field into the local `tasks` table and `gqt` models.
    - **SQLite Sorting:** Updated `get_tasks` to use a robust `ORDER BY` clause: `CASE WHEN t.position IS NULL THEN 1 ELSE 0 END, t.position ASC, t.creation_date ASC`. This ensures synced tasks follow server ordering while new local tasks (with `NULL` position) correctly sort to the bottom.
    - **Infrastructure:** Switched `gqt` to use the local version of the expanded library and migrated the database schema to store new fields.

## 2026-05-14
- **Sync Fix:** Resolved a "Serialization Error" during sync.
    - **Data Types:** Corrected the `duration` field in `gqueues-api-rs` from `String` to `u32` to match the integer values returned by the GQueues API.
- **UI Consistency:** Fixed a bug where new tasks would "disappear" from the immediate view after pressing 'Tab'.
    - **Local Ordering:** Introduced a `local_order` floating-point field to the `tasks` table.
    - **Interpolation:** Implemented an interpolation algorithm in `main.rs` to calculate a `local_order` value for new tasks based on their neighbors. This ensures they stay exactly where they were created until replaced by server-side `position` data.
    - **Database Sorting:** Updated `get_tasks` to prioritize `position` (from server) and then `local_order`, with a fallback for `NULL` values.

## 2026-05-16
- **CLI Task Creation:** Implemented rapid task entry via the command line with GQueues Quick Add Syntax support.
    - **Infrastructure:** Integrated `clap` 4.6.1 for argument parsing and established a modular `src/commands` structure.
    - **Quick Add:** Introduced `Operation::CreateQuick` to trigger server-side syntax parsing (`parseQuickAddSyntax=True`) in the Sync Engine.
    - **Dual-Mode Boot:** Refactored `main.rs` to support both TUI and CLI execution, sharing core initialization logic.
    - **Immediate Sync:** Enabled automatic background synchronization immediately after CLI task entry.
    - **User Feedback:** Integrated `indicatif` for dynamic terminal spinners and provided clear status updates, including a versioned header.
    - **UI Refinement:** Ensured the CLI clears the spinner before final confirmation for a clean output.
    - **Type Safety:** Resolved numerous model mismatches across `src/ui.rs`, `src/sync.rs`, and `src/main.rs` following library updates.
