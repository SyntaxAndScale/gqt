# Gqueues TUI (gqt)

A terminal-based user interface (TUI) for managing Gqueues tasks, built with Rust and Ratatui.

## Features

- **Three-Pane Layout:** Sidebar for Queues, Task List, and Detail View.
- **Keyboard Centric:** Navigate and manage tasks entirely from the terminal.
- **Real-time API Integration:** Communicates with the Gqueues Beta REST API.
- **Async Execution:** Responsive UI with background data fetching.

## Navigation

- `Tab` / `Shift-Tab`: Switch focus between panes (Queues, Tasks, Details).
- `Up` / `Down`: Navigate through lists.
- `Enter` (Queues pane): Load tasks for the selected queue.
- `q`: Quit the application.

## Setup

1. Ensure you have Rust and Cargo installed.
2. Configure your Gqueues credentials in `.gemini/settings.local.json`:
   ```json
   {
     "gqueues": {
       "apiEndpoint": "https://api.gqueues.com",
       "accessToken": "your_access_token_here"
     }
   }
   ```
3. Run the application:
   ```bash
   cargo run
   ```

## Development

This project is currently a prototype. Future plans include:
- CRUD operations (Create, Update, Delete).
- Offline-first caching with CRDT-based synchronization.
- Enhanced keyboard shortcuts and vim-like navigation.

## License

Copyright (c) 2026 Syntax & Scale, LLC. All rights reserved.

This software and its associated documentation are the proprietary property 
of Syntax & Scale, LLC. 

Unauthorized copying, distribution, or modification of this software, 
via any medium, is strictly prohibited. 

For inquiries regarding commercial licensing or acquisition, please contact 
contact@syntaxandscale.com .