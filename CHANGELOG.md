# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

### [0.1.1] / [0.2.0] (Next Release) -> Iteration 9 & Polish
- Removed:
  * Removed recursive split tree rendering for stability.
- Added:
  * **Agent and Pageant Authentication Chain**: SSH connections now try Pageant/SSH agent identities first, then a configured private key, then default keys from `~/.ssh`, and finally password authentication.
  * **Password Prompt Fallback**: If agent and key-based authentication do not succeed and no password is stored in the profile, the terminal now prompts for the password with a masked input and reveal toggle.
  * **Per-Connection Terminal Type**: Connection profiles can now store their own PTY terminal type such as `xterm`, `xterm-color`, or `xterm-256color`.
  * **Custom Config Directory**: Added a Settings option to choose the directory used for `settings.json`, `profiles.json`, and `autocomplete.json`, with automatic migration of existing files.
  * **Terminal Selection Context Menu**: Right-clicking inside the terminal now opens a custom context menu with `Copy`, `Paste`, and `Search on Web`. Web search opens the default browser with the selected text.
  * **SFTP Visual Browser**: Right-click context option on any connected tab to open an interactive file tree modal.
  * **Remote File Actions**: Supported recursive directory browsing, downloads to local machine, and fast uploads through Base64 payload encoding over the active SSH channel.
  * **Visual Shell Autocomplete**: Added a floating overlay widget to `xterm.js` that tracks in-progress text and suggests standard Linux/Bash commands (e.g. `systemctl`, `docker`). Includes Up/Down arrow selection and Tab injection.
    * Supports parameter autocomplete for commands like `systemctl`, `docker`, etc., generated on the fly after pressing space.
    * The algorithm intelligently hides suggestions if the user's current parameter is already complete.
    * Command dictionary was extracted to a modifiable `autocomplete.json` file in the application's config folder.
    * Learning new commands: Unknown words can now be silently saved to the local dictionary using the `Shift+Tab` keyboard shortcut. This frees up the native `Tab` key for natural Linux path completion (e.g. `/var/log`).
    * Implemented Levenshtein distance for fuzzy matching and improved suggestion relevance.
### Changed
* Reworked the connection editor into a more compact two-pane layout so additional connection options can be added without the form becoming visually heavy.
* Added a toggle for smart command autocomplete (`Enable Smart Autocomplete`) in the `Terminal` section of the Settings window, responding instantly without reconnecting sessions.
* The status bar now displays a small hint recommending the relevant keyboard shortcut.

### Fixed
* Fixed the runtime password prompt so clicking into the password field no longer hands focus back to the terminal wrapper and prevents typing.
* Fixed the WSL PTY path to propagate the selected connection terminal type via `TERM` instead of silently falling back to the default terminal setting.
* Fixed stale connection profile usage during connect and reconnect flows so updated per-connection terminal types are reloaded from storage instead of reusing an older in-memory snapshot.
* Fixed SSH PTY initialization to also export `TERM` explicitly on the SSH channel and show the requested terminal type in the terminal log during connection setup for easier verification.
* Fixed a regression where the spacebar stopped working in the terminal because the outer terminal container intercepted the `Space` key while handling focus for the custom context menu wrapper.
* The Xterm terminal field border was adding a bottom offset that clipped the visualization of the last text line when the window was maximized. The issue was resolved by moving padding below a clean flex-wrapper (including `box-sizing: border-box`), which corrects the miscalculations from Xterm's internal `FitAddon` algorithm.
* Line gutters would sometimes drift from the actual text due to flexbox formatting and browser rounding errors. Line numbers now use precise decimal cell height from the native grid and absolute positioning (`top: Npx` for each section), which completely eliminates cumulative height drift.
* The floating autocomplete popup no longer jumps high up if it has only one item near the bottom edge of the window. Position is now controlled by real dynamic height estimation via Boundary Box rendering.
* Navigation in bash history (arrow up/down) no longer falsely triggers the floating Autocomplete window, thus no longer blocking subsequent arrow key presses.
* The WebGL renderer remains active for proper rendering of icons and Powerline/Nerd font glyphs in `oh-my-zsh`, but after history navigation the terminal now explicitly clears the texture atlas and repaints the viewport. This replaces the effect that previously required a window resize.
* The terminal now also re-measures itself after fonts are loaded and prefers Powerline/Nerd Mono fonts so that the ZSH prompt uses stable cell metrics.
* "Ghosting" artifacts when navigating bash history are finally resolved. The issue was caused by exotic characters (e.g. unicode arrows) that the browser natively treats as 1 pixel wide. The `unicode11` addon was installed to correctly map ZSH symbols (as 2-pixel wide characters), so backspace when deleting history finally reaches the first remaining character of the previous long word.
* Fixed a fatal bug causing a blank black screen on terminal addon instantiation injection failure. The main rendering loop and session startup now successfully recover even if libraries fail.
* Fixed a race condition causing a blank screen after connection. The `ssh-output` listener is now registered **before** calling `connect_ssh`, preventing loss of initial server output (MOTD/prompt).
* Implemented proper connection teardown (e.g. when pressing `Ctrl+D` or typing `exit`). The status bar and "Tab" are no longer stuck in a false "Connected" state, but correctly destroy the session after channel termination from the Ubuntu/Linux server.

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
