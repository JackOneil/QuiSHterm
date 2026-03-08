# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-08

### Added
- **Core SSH Functionality**: Complete integration of SSH terminal using Rust backend and xterm.js frontend.
- **Connection Manager**: UI for adding, editing, and establishing SSH connections, including key-based auth support.
- **QuickConnect Sidebar**: Right sidebar panel for quickly launching and organizing saved connections into customizable color-coded folders. Drag-to-move support using "move mode".
- **Sidebar Context Menu**: Right-click context menus added to the right sidebar for connecting, editing, and deleting connections or folders.
- **Dynamic PTY Resizing**: Remote PTY automatically resizes when the local terminal window size changes.
- **Custom Highlighting**: Built-in and user-configurable regex-based text highlighting in terminal output (e.g. coloring ERRORs red).
- **Status Bar**: A dynamic bottom UI panel tracking TX/RX byte data, SSH server information, and visual connection status.
- **Settings Architecture**: Resizable settings modal with options stored persistently in the app data directory.
- **Terminal Settings**: Ability to configure scrollback buffer size (up to 100000 lines) and display a reactive line number gutter.
- **Tabbed Interface**: Manage multiple active SSH connections with the ability to rename sessions and quickly switch between them without losing state or scroll history.
- **Tab Context Menu Improvements**: Added a 'Reconnect' action to the session tab context menu allowing users to quickly restart disconnected or frozen sessions.
- **Split Panes**: Completely refactored the layout engine to support vertical and horizontal terminal splitting! Right-click or use the pane-controls to split terminals infinitely and view multiple connections side-by-side within a single tab.

### Changed
- Refactored frontend structure for maximum UI responsiveness using SvelteKit.
- Moved dragging behavior in sidebar to a more resilient click-to-move pattern avoiding bugs in webview drag interactions.
- Replaced hard-coded highlight dictionary string replacements with proper regex evaluations that support special characters.
- Disabled the global native browser right-click context menu to enforce an app-like feel, while retaining native right-click copy/paste strictly inside the xterm canvas.

### Fixed
- Fixed issue where the settings modal would incorrectly close when accidentally dragging off the resize bounds and releasing the mouse check.
- Fixed line gutter rendering issues to perfectly match `xterm.js` row height and properly hook into the native `onScroll` events.
- Fixed broken SSH connections due to outdated cipher negotiation routines by integrating modern openssl support to the build pipeline for Windows targets.
- Fixed tab switching logic that inadvertently reconnected active sessions instead of persisting them via CSS state changes.
