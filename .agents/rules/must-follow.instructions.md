# System Instructions for AI Assistants (Core Conventions)

This file defines rules for any future modifications and interventions in the `QuiSHterm` project. Every time you start working on this project, **YOU MUST** follow these guidelines.

## 1. Documentation Maintenance (CHANGELOG and README)
- **Always update `CHANGELOG.md`:** As soon as you add new functionality, modify an existing feature, or fix a bug, you **must** write a clear note in the `CHANGELOG.md` file.
  - Use the appropriate section (Added, Changed, Deprecated, Removed, Fixed, Security).
  - The project strictly follows the **Keep a Changelog** format and **Semantic Versioning**.
- **Always keep `README.md` up-to-date:** If your modification changes how the project runs, builds, or if you add a completely new core feature (e.g. a new UI window), update the *Features* or *Build Instructions* section in `README.md`.

## 2. Architecture and Project Structure (Where to Find What)
This project is built on the **Tauri** framework (Rust backend + SvelteKit frontend). If you lose context, here's a quick orientation:

### Frontend (SvelteKit + Vite)
- Folder: `/src`
- **User Interface:** All main components are in `/src/lib/components/`.
  - `ConnectionManager.svelte`: Modal window for adding/editing SSH profiles (IP, user, keys).
  - `SettingsManager.svelte`: Modal window for global settings (Scrollback buffer, syntax highlighting).
  - `QuickConnect.svelte`: Right-side panel for quick connection and organizing profiles into folders.
  - `TerminalArea.svelte`: The actual xterm.js wrapper for rendering the SSH terminal.
- **Main Canvas:** `/src/routes/+page.svelte` controls the main application flow — maintains open tab states, status bar, sends/receives data from components, and communicates with the Rust backend via Tauri events.

### Backend (Rust + Tauri)
- Folder: `/src-tauri`
- **Tauri Configuration:** `tauri.conf.json`, `Cargo.toml`. This is where you change the app name, ID, and dependencies.
- **Logic and System:** `/src-tauri/src/`
  - `main.rs`: Entry point that simply calls the library.
  - `lib.rs`: Registration of Tauri commands and backend application startup.
  - `ssh_manager.rs`: Core SSH connection logic (using the `ssh2` library). Handles PTY (resize), shell, reading and writing (`ssh-stats`, `ssh-output` event emission over network buffers).
  - `settings_storage.rs`: File system operations connected to the application data folder (reading and saving `settings.json` and `profiles.json`).

## 3. Development Flow and Build
- Command for development with instant hot-reload: `npm run tauri dev`
- **Production Build / Windows (via Docker):** 
  The project includes `Dockerfile.builder` for seamless cross-compilation from any Linux environment to Windows `.exe`. Always rely on this build method.
  Command: `docker run --rm -v $(pwd):/app -e PKG_CONFIG_ALLOW_CROSS=1 -w /app tauri-builder npm run tauri build -- --target x86_64-pc-windows-gnu`
- **IMPORTANT RULE:** After completing each development iteration (functional block) **YOU MUST ALWAYS** run the Docker build (command above) to verify compilation and create the final EXE package.

## 4. Working in Autonomous/Agent Mode
- Prefer small, isolated modifications over massive component rewrites.
- If you perform refactoring (e.g. adding ARIA tags to UI or rewriting a component), **always test Svelte compilation** before final submission (`npm run build`).
- Do not create nested bash commands like `echo >` for file creation; use internal file system/write tools.
- **IMPORTANT: All written code and third-party process integrations (e.g. WSL, file system) MUST ALWAYS include built-in DEBUG LOGGING.**
  - The application has an internal toggle to enable "Debug Mode" for the user.
  - Use the method `crate::settings_storage::log_debug(&app_handle, "Message");`, which writes the message to console/UI if debug mode is enabled, dramatically speeding up bug resolution. Every new function must log its states and exceptions.

## 5. Language Rule: English Only
- **All modifications, documentation, and code comments MUST be in English**, regardless of the language in which user prompts are written.
- This includes: terminal output texts, status messages, error messages, all UI labels, and documentation (README, CHANGELOG, comments).
- User prompts may be in any language, but the implementation must always be in English.
- When updating CHANGELOG or README, translate all content to English.
- When adding new features, all user-facing text must use English.
