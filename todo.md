# TODO: Gqueues TUI

## Initial Setup
- [x] Create documentation files (GEMINI.md, prd.md, spec.md, journal.md).
- [x] Initialize Rust project (`cargo init`).
- [x] Initialize Git and configure `.gitignore`.
- [ ] Research Gqueues API for authentication and task modification.

## Persistence & Sync
- [x] Phase 1: Setup & Schema (0.1.2)
- [x] Phase 2: Local CRUD (0.1.3)
- [x] Phase 3: Sync Engine (0.1.4)
    - [x] Handle idempotency keys in transaction log.
    - [x] Implement background push/pull.
    - [x] Fix decoding errors and add logging.
- [x] Phase 4: Sync Optimization (0.1.6)
    - [x] Prioritize active queue.
    - [x] Metadata-first sync (lastModified check).
    - [x] Respect Retry-After header.
    - [x] XDG config migration.

## Architecture
- [x] Refactor API client into a decoupled module for future extraction as a standalone library.

## UI & Navigation
- [x] Implement categorized/grouped queues in the left pane.
    - [x] Add category support to database schema.
    - [x] Implement collapsible/expandable category headers.
    - [x] Update UI to render grouped list.

## Future Work
- [ ] Implement GQueues API authentication workflow (user input for API key).
- [ ] Change configuration file format from .json to .toml
- [ ] When initializing a database for the first time, if a gqueues api key is available, then the user experience should be that the list of queues is queried and displayed, but the only tasks that are initially fetched are for the Inbox queue. The other queues can/will be queried either as they are selected, or as part of a background sync. The sync process should display the number of queues and/or estimated tasks remain to be queried.
- [ ] When the program is launched and there is no existing database or configuration, the user should be prompted to either (a) input a Gqueues API key to sync from an existing account (b) specify the path of an existing local gqt database, (c) create a new local-only GQT database (and the user should specify the  path with an XDG default path suggested)
- [ ] The task detail on the right pane should include the url to the web client's task so a user can click on the link (in some terminal emulators) to go to the official gqueues web client (assuming the task is sync'd to gqueues - if not, then no url should be displayed)
- [ ] when the status bar shows a successful sync, it should include text about the time it was last sync'd in ISO 8601 timestamp format adjusted for the user's local (system) time