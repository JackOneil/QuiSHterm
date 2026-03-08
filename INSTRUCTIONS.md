# System Instructions pro AI asistenty (Základní konvence)

Tento soubor definuje pravidla pro jakékoliv budoucí úpravy a zásahy do projektu `QuiSHterm`. Pokaždé, když začínáte práci na tomto projektu, **MUSÍTE** se řídit následujícími pokyny.

## 1. Udržování dokumentace (CHANGELOG a README)
- **Vždy aktualizovat `CHANGELOG.md`:** Jakmile přidáte novou funkcionalitu, upravíte stávající funkci, nebo opravíte bug, **musíte** zapsat srozumitelný záznam do souboru `CHANGELOG.md`. 
  - Použijte odpovídající sekci (Added, Changed, Deprecated, Removed, Fixed, Security).
  - Projekt striktně dodržuje format **Keep a Changelog** a **Semantic Versioning**.
- **Vždy udržovat `README.md` aktuální:** Pokud úprava mění způsob, jakým se projekt spouští, staví (build), nebo pokud přidáváte zcela novou stěžejní funkci (např. nové UI okno), aktualizujte sekci *Features* nebo *Build Instructions* v `README.md`.

## 2. Architektura a struktura projektu (Kde co najít)
Tento projekt je stavěný na frameworku **Tauri** (Rust backend + SvelteKit frontend). Pokud ztratíte kontext, zde je rychlá orientace:

### Frontend (SvelteKit + Vite)
- Složka: `/src`
- **Uživatelské rozhraní:** Všechny hlavní komponenty jsou v `/src/lib/components/`.
  - `ConnectionManager.svelte`: Modal okno pro přidání/úpravu SSH profilu (IP, uživatel, klíče).
  - `SettingsManager.svelte`: Modal okno pro globální nastavení (Scrollback buffer, zvýrazňování syntaxe).
  - `QuickConnect.svelte`: Boční (pravý) panel pro rychlé připojení a organizaci profilů do složek.
  - `TerminalArea.svelte`: Samotný xterm.js wrapper pro vykreslování SSH terminálu.
- **Hlavní plátno:** `/src/routes/+page.svelte` řídí hlavní flow aplikace — drží stavy otevřených tabů, status bar, posílá data z/do komponent a komunikuje s Rust backendem přes Tauri events.

### Backend (Rust + Tauri)
- Složka: `/src-tauri`
- **Tauri konfigurace:** `tauri.conf.json`, `Cargo.toml`. Zde se mění jméno, ID aplikace a dependencies.
- **Logika a systém:** `/src-tauri/src/`
  - `main.rs`: Vstupní bod, pouze volá knihovnu.
  - `lib.rs`: Registrace Tauri commands a start backend aplikační logiky.
  - `ssh_manager.rs`: Jádro SSH připojení (knižnice `ssh2`). Řeší PTY (resize), shell, čtení a zápis (`ssh-stats`, `ssh-output` emitting na síťových bufferech).
  - `settings_storage.rs`: FS operace napojené na aplikační data složku (vyčítání a ukládání `settings.json` a `profiles.json`).

## 3. Vývojové flow a build
- Příkaz pro vývoj a instantní hot-reload: `npm run tauri dev`
- **Build pro produkci / Windows (skrz Docker):** 
  Projekt obsahuje `Dockerfile.builder` pro bezproblémový build z jakéhokoliv Linux prostředí na Windows `.exe`. Vždy spoléhejte na tento způsob buildu.
  Příkaz: `docker run --rm -v $(pwd):/app -e PKG_CONFIG_ALLOW_CROSS=1 -w /app tauri-builder npm run tauri build -- --target x86_64-pc-windows-gnu`

## 4. Práce v agentním / autonomním módu
- Preferujte malé a izolované úpravy namísto obřích přepisů komponent.
- Pokud provádíte refaktorování (např. přidání ARIA tagů do UI nebo přepis komponenty), **vždy otestujte svelte kompilaci** před finálním odevzdáním (`npm run build`).
- Nevytvářejte vnořené bash přikazy typu `echo >` pro tvorbu souborů, používejte interní FS/write nástroje.
- **DŮLEŽITÉ: Napsaný kód a integrace third-party procesů (např. WSL, filesystém) MUSÍ MÍT VŽDY ZABUDOVANÝ DEBUG LOGGING.**
  - Aplikace obsahuje interní toggle pro zapnutí "Debug Mode" u uživatele.
  - Využívejte metodu `crate::settings_storage::log_debug(&app_handle, "Zpráva");`, která v případně zapnutého debug módu zapíše zprávu do konzole/UI a dramaticky tím zrychluje řešení bugů. Každá nová future musí logovat své stavy a výjimky.
