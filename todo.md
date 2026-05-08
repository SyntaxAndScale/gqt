# TODO: Gqueues TUI

## Initial Setup
- [x] Create documentation files (GEMINI.md, prd.md, spec.md, journal.md).
- [x] Initialize Rust project (`cargo init`).
- [x] Initialize Git and configure `.gitignore`.
- [X] Research Gqueues API for authentication and task modification.

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
- [x] Phase 5: Initial Sync UX & Progress (0.1.11)
    - [x] Add `tasks_fetched` tracking to database.
    - [x] Implement Inbox-first task fetching.
    - [x] Implement background queue-by-queue sync.
    - [x] Display sync progress (remaining queues) in status bar.

## Architecture
- [x] Refactor API client into a decoupled module for future extraction as a standalone library.

## UI & Navigation
- [x] Implement categorized/grouped queues in the left pane.
    - [x] Add category support to database schema.
    - [x] Implement collapsible/expandable category headers.
    - [x] Update UI to render grouped list.
    - [x] Implement dynamic grouping by API `categoryName`.

## Future Work
- [X] Implement GQueues API authentication workflow (user input for API key).
- [x] Change configuration file format from .json to .toml
- [x] When initializing a database for the first time, if a gqueues api key is available, then the user experience should be that the list of queues is queried and displayed, but the only tasks that are initially fetched are for the Inbox queue. The other queues can/will be queried either as they are selected, or as part of a background sync. The sync process should display the number of queues and/or estimated tasks remain to be queried.
- [x] When the program is launched and there is no existing database or configuration, the user should be prompted to either (a) input a Gqueues API key to sync from an existing account (b) specify the path of an existing local gqt database, (c) create a new local-only GQT database (and the user should specify the  path with an XDG default path suggested)
- [x] Add a '?' command to show a help screen.
- [x] I want the bottom line of the TUI to always show when a sync is happening and the status of that sync.
- [ ] Update The task detail on the right pane should include the url to the web client's task so a user can click on the link (in some terminal emulators) to go to the official gqueues web client (assuming the task is sync'd to gqueues - if not, then no url should be displayed)
- [ ] when the status bar shows a successful sync, it should include text about the time it was last sync'd in ISO 8601 timestamp format adjusted for the user's local (system) time
- [x] The list of queues and tasks within a queue are sometimes longer than can be displayed vertically within the TUI. We need to implement scrolling for when the user scrolls further down in any of the panes than can be displayed within the current vertical dimensions of a pane. 
- [ ] I noticed that when I deleted a task in the Gqueues web UI that it didn't delete in the gqt TUI. I dont know if an API call will list the deleted tasks, but we will need someway to apply deletions from the official gqueues remote database to the local database.
- [x] Implement keyboard configuration customization. The default keyboard controls should match the web version of gqueues as closely as possible except where they would be impractical or impossible within a TUI envrionment. They should be configurable via a `config.toml` stored in the appropriate XDG path and all the defaults should be pre-loaded in the configuration file on first run. 
- [x] Update the right pane (task detail) to show each of the task's Tags, Title, Assignee, Task Creation Date, Due Date (from the API's 'rawDate'), Repeat information (if repeating is not false), then Notes.
- [ ] Phase 6: Advanced HTML Rendering (Future)
    - [ ] Map HTML tags (`<b>`, `<u>`, etc.) directly to Ratatui `Span` styles instead of stripping them.
- [x] If a task displayed in the center panel has sub-tasks, it should include a expand/collapse arrow that prepends the "[ ]" of the task. Pressing the spacebar on the task should expand/collapse to show the sub-tasks.
- [x] When navigating the category list, make it so that the space bar can be pressed to expand/collapse a category
- [ ] Implement the shortcut keys for hiding/showing the left and right panels as VSCode uses for the left and right side panels.
- [x] Implment a shortcut key press to 'Sync now' which should basically force a sync between the local database and the gqueues web service.
- [ ] Re-design and update formatting of task details
- [ ] Re-design and update the formatting of the help screen '?'
- [ ] When first loading gqt, the tasks from the Inbox queue are shown in the center pane (correctly) but the navigation pane should also have the Inbox queue selected. Currently it shows the Category being selected. It should expand the necessary Category (if applicable) and show the Inbox queue selected.

## TODO Before Open Sourcing
- [ ] Choose and apply a license based on what is legally viable by what the imported crates use
- [ ] Review and clean up the most egregious AI slop
- [ ] Update readme.md to be clear this is AI generated code, the roadmap, installation, usage, vision, etc.
- [ ] Fix journal dates to be based on git history
- [ ] Update the help screen with the github url
- [ ] Update `spec.md` with the latest information based on current state of code. Should include urls to api documentation and keyboard shortcut documentation. 
- [ ] Add a disclaimer to help screen and readme.md that this is not an official gqueues product
- [ ] 
