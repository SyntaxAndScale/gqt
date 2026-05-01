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
- **GQueues API Module:** A decoupled module (`src/gqueues/`) containing the API client and models, designed for future library extraction.
- **Sync Engine:** Manages the local cache and handles background synchronization with the Gqueues API.
- **CRDT Strategy:** 
    - Operations (Create, Update, Delete) are stored in a local **Transaction Log**.
    - Each transaction has a timestamp and a unique ID.
    - Local state is a projection of the transaction log.
    - Sync engine attempts to push transactions to the API sequentially.
    - Conflict resolution: Last Write Wins (LWW) based on client timestamps for now, until more complex CRDT needs arise.


## Layout Components
1. **Left Pane (Queues):** Navigation between different Gqueues lists.
2. **Center Pane (Tasks):** Filtered list of tasks for the selected queue.
3. **Right Pane (Details):** Metadata, notes, and subtasks for the selected task.
