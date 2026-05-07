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
- [x] Fix task routing to active queue and improve local persistence.
- [ ] Phase 5: Initial Sync UX & Progress (0.1.11)
    - [ ] Add `tasks_fetched` tracking to database.
    - [ ] Implement Inbox-first task fetching.
    - [ ] Implement background queue-by-queue sync.
    - [ ] Display sync progress (remaining queues) in status bar.

## Architecture
- [x] Refactor API client into a decoupled module for future extraction as a standalone library.

## UI & Navigation
- [x] Implement categorized/grouped queues in the left pane.
    - [x] Add category support to database schema.
    - [x] Implement collapsible/expandable category headers.
    - [x] Update UI to render grouped list.

## Future Work
- [ ] Implement GQueues API authentication workflow (user input for API key).
- [x] Change configuration file format from .json to .toml
- [x] When initializing a database for the first time, if a gqueues api key is available, then the user experience should be that the list of queues is queried and displayed, but the only tasks that are initially fetched are for the Inbox queue. The other queues can/will be queried either as they are selected, or as part of a background sync. The sync process should display the number of queues and/or estimated tasks remain to be queried.
- [x] When the program is launched and there is no existing database or configuration, the user should be prompted to either (a) input a Gqueues API key to sync from an existing account (b) specify the path of an existing local gqt database, (c) create a new local-only GQT database (and the user should specify the  path with an XDG default path suggested)

- [ ] Update The task detail on the right pane should include the url to the web client's task so a user can click on the link (in some terminal emulators) to go to the official gqueues web client (assuming the task is sync'd to gqueues - if not, then no url should be displayed)
- [ ] when the status bar shows a successful sync, it should include text about the time it was last sync'd in ISO 8601 timestamp format adjusted for the user's local (system) time
- [x] The list of queues and tasks within a queue are sometimes longer than can be displayed vertically within the TUI. We need to implement scrolling for when the user scrolls further down in any of the panes than can be displayed within the current vertical dimensions of a pane. 
- [ ] I noticed that when I deleted a task in the Gqueues web UI that it didn't delete in the gqt TUI. I dont know if an API call will list the deleted tasks, but we will need someway to apply deletions from the official gqueues remote database to the local database.
- [x] Implement keyboard configuration customization. The default keyboard controls should match the web version of gqueues as closely as possible except where they would be impractical or impossible within a TUI envrionment. They should be configurable via a `config.toml` stored in the appropriate XDG path and all the defaults should be pre-loaded in the configuration file on first run. 
- [x] Update the right pane (task detail) to show each of the task's Tags, Title, Assignee, Task Creation Date, Due Date (from the API's 'rawDate'), Repeat information (if repeating is not false), then Notes.
- [ ] Phase 6: Advanced HTML Rendering (Future)
    - [ ] Map HTML tags (`<b>`, `<u>`, etc.) directly to Ratatui `Span` styles instead of stripping them.
- [x] If a task displayed in the center panel has sub-tasks, it should include a expand/collapse arrow that prepends the "[ ]" of the task.
- [x] When navigating the category list, make it so that the space bar can be pressed to expand/collapse a category
- [ ] Implement the shortcut keys for hiding/showing the left and right panels
- [x] Implment a shortcut key press to 'Sync now' which shoudl basically force a sync between the local database and the gqueues web service.
- [x] BUG: When adding a new task to the local DB, it still doesn't appear to be sync'ing to the gqueues web service. 
- [ ] BUG: The force-sync keyboard shortcut 's' conflicts with the gqueues reference for keyboard shortcuts. Lets change the force-sync to be 'control-s'
- [ ] BUG: I noticed that when starting gqt when there is not a local database, that it did not begin sync'ing automatically. I had to initiate a force sync in order to get the local database started. RULE: if there is a gqueues api key in the config.toml file, but no local database file, then we should initiate a sync.

# Before open sourcing
- [ ] Choose a license
    + [ ] Review compatible licenses given the crates used in the project and their licenses
- [ ] Add a '?' command to show information and help - it should contain url of the repo, keyboard commands (with unimplemented ones grey'd out), and vesion information
- [ ] Add a command to open config.toml so it is easy to find and update the api key manually if necessary
- [ ] Split out `gqueues-api-rs`, create an open source repo for it, license it, and update gqt to reference it on github
- [ ] 1st pass basic/quick review to clean up egregious AI slop
- [ ] Fix up journal.md so that the dates are informed by the git history.
- [ ] Make sure there are no sensitive files or logs in commit history and are in .gitignore
- [ ] Write up a rough roadmap that includes all the workflows supported by shortcut keys and the sequence / order in which they are intended to be implemented.