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
- [ ] Phase 4: Sync Optimization (0.1.6)
    - [ ] Prioritize active queue.
    - [ ] Metadata-first sync (lastModified check).
    - [ ] Respect Retry-After header.

## Architecture
- [x] Refactor API client into a decoupled module for future extraction as a standalone library.

## Future Work
- [ ] Implement GQueues API authentication workflow (user input for API key).
- [ ] Change configuration file format from .json to .toml