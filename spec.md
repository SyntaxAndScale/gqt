# Technical Specification: Gqueues TUI (gqt)

## Technology Stack
- **Language:** Rust
- **UI Framework:** Ratatui
## API Communication
- **Base Endpoint:** `https://api.gqueues.com/v0`
- **Authentication:** Bearer Token via `Authorization` header.
- **Actions (Beta):**
    - `getQueues`: (GET) Fetch queues. Optional query param `scope` (`personal`, `team`, `shared`).
    - `getActiveTasks`: (GET) Fetch active tasks for a specific `queueKey`. Supports `limit`, `cursor`, `includeSnoozed`.
    - `createTask`: (POST) Create tasks. Each instruction in `instructions` array can have `text`, `queueKey`, `notes`, `parseQuickAddSyntax`.
- **Headers:**
    - `Idempotency-Key`: Required for all POST operations.
    - `Content-Type: application/json`: For POST operations.
- **Note:** `updateTask` and `deleteTask` are not currently available in the public Beta API.

## Architecture
- **TUI Layer:** Handles rendering and user input via Ratatui (`src/ui.rs`).
- **State Management:** A central state object managing the current view and task data (`src/app.rs`).
- **GQueues API Module:** A decoupled module (`src/gqueues/`) containing the API client and models.
- **Persistence Layer:** SQLite database using `rusqlite`, following **XDG best practices** (stored in `$XDG_DATA_HOME/gqt`).
- **Sync Engine:** A background task using `tokio::select!` and exponential backoff to reconcile local and remote states.
- **Robust Identity Mapping:** Uses internal UUIDs (`local_id`) for all database relationships to ensure data integrity for local-only tasks, with NULL-safe resolution for GQueues `remote_key` promotion.
- **CRDT Strategy:** 
    - Operations (Create, Update, Delete) are stored in a local **Transaction Log**.
    - Each transaction has a timestamp and a unique ID.
    - Local state is a projection of the transaction log.
    - Sync engine attempts to push transactions to the API sequentially.
    - Conflict resolution: Last Write Wins (LWW) based on client timestamps for now, until more complex CRDT needs arise.


## Layout Components
1. **Left Pane (Queues):** Categorized and collapsible navigation. Queues are grouped by API `categoryName` or `teamName`, with support for "Personal" and "Archive" sorting priorities.
2. **Center Pane (Tasks):** Filtered list of tasks for the selected queue. Supports hierarchical sub-tasks.
3. **Right Pane (Details):** Metadata, notes, and subtasks for the selected task. Includes rich formatting for tags and dates.
4. **Help Modal:** An interactive overlay (`?`) displaying app metadata and dynamic keybindings.
