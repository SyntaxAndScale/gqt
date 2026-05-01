# TODO: Gqueues TUI

## Initial Setup
- [x] Create documentation files (GEMINI.md, prd.md, spec.md, journal.md).
- [x] Initialize Rust project (`cargo init`).
- [x] Initialize Git and configure `.gitignore`.
- [ ] Research Gqueues API for authentication and task modification.

## Core Implementation
- [x] Basic TUI layout implementation with Ratatui.
- [x] Navigation logic between panes.
- [x] Local storage/caching layer. (In progress: Using memory for now)
- [x] API integration for fetching tasks.
- [ ] CRUD: Create task implementation.
- [x] CRUD: Read/List tasks implementation.
- [ ] CRUD: Update task implementation.
- [ ] CRUD: Delete task implementation.

## Architecture
- [x] Refactor API client into a decoupled module for future extraction as a standalone library.
