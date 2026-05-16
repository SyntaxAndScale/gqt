# Gqueues TUI (gqt)

A terminal user interface for managing GQueues tasks. Built with Rust and Ratatui.

> **Disclaimer:** This is an unofficial project and is NOT commercially affiliated with GQueues. This software was developed with from Google Gemini CLI and has not been thoroughly reviewed for security or correctness. The software is considered a  prototype. Backup your data and use at your own risk.

## Features

- **Offline-First:** All data is cached in a local SQLite database. Work anywhere, sync when online.
- **Background Sync:** A dedicated engine reconciles local changes and fetches remote updates without blocking the UI.
- **Three-Pane Layout:** Intuitive navigation between Queues, Tasks, and Task Details.
- **Hierarchical Tasks:** Support for sub-tasks with collapsible/expandable nodes.
- **Web-Parity Keyboard Shortcuts:** Familiar navigation for GQueues power users. Not all workflows implemented yet.
- **Customizable:** Configurable keybindings

## Installation
Requires Rust. 

```bash
# Clone the repository
git clone https://github.com/SyntaxAndScale/gqt
cd gqt

# Build and run
cargo run --release
```

## Usage

### TUI Mode
Simply run the binary to launch the full terminal interface:
```bash
gqt
```

### CLI Quick Add
Create a task instantly from your terminal using GQueues Quick Add Syntax:
```bash
gqt -i "Buy milk tomorrow @5pm #errands [Personal] :: Remember the organic one"
```
The CLI will save the task locally and immediately sync it with the GQueues API.

## Setup

On the first launch, `gqt` will guide you through an interactive setup wizard:
1. **API Key:** You will be prompted to enter your GQueues API Key (available in your account settings).
2. **Initial Sync:** The application will immediately fetch your queue names and your Inbox tasks so you can get started instantly.

## Default Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `?` | Show Help Modal (Dynamic Reference) |
| `Tab` | Cycle through panes |
| `j` / `k` | Navigate lists |
| `Space` | Toggle Category / Sub-task expansion |
| `s` | Sync Now (Manual Trigger) |
| `g` then `i` | Go to Inbox |
| `Esc` | Cancel / Close Modal |
| `Ctrl-c` | Quit |

## Roadmap
- [ ] Task Creation
- [ ] Task Completion (`c`) and Archiving (`Shift-C`).
- [ ] Task Deletion.
- [ ] Task Editing.
- [ ] Rich TUI rendering (mapping GQueues formatting to TUI styles).

## Someday / Maybe
- [ ] Command palette 
- [ ] Command-line CRUD and sync commands
- [ ] Custom Themes
- [ ] Database path configuration
- [ ] Extensions / Plug-ins for custom workflows

## Contributing
TBD

## License
*License choice pending.*

