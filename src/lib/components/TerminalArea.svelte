<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { WebglAddon } from "@xterm/addon-webgl";
  import { Unicode11Addon } from "@xterm/addon-unicode11";
  import AutocompleteOverlay from "./AutocompleteOverlay.svelte";
  import "@xterm/xterm/css/xterm.css";

  export let sessionId: string;
  export let profile: any;

  let terminalContainer: HTMLElement;
  let term: Terminal;
  let fitAddon: FitAddon;
  let webglAddon: WebglAddon | null = null;
  
  let unlistenOutput: UnlistenFn | null = null;
  let isConnecting = true;
  let connectionError = "";
  let highlights: any[] = [];
  let scrollback = 10000;
  let showLineNumbers = false;
  let enableAutocomplete = true;
  let lineNumberEl: HTMLElement;
  let settingsCheckInterval: ReturnType<typeof setInterval> | null = null;
  let contextMenuEl: HTMLElement;
  let resizeObserver: ResizeObserver | null = null;

  let terminalContextMenu = {
    visible: false,
    x: 0,
    y: 0,
    hasSelection: false,
    selectionText: ""
  };

  // Autocomplete state
  let currentWord = "";
  let showAutocomplete = false;
  let autocompleteX = 0;
  let autocompleteY = 0;
  let filteredSuggestions: string[] = [];
  let selectedAutocompleteIndex = 0;
  
  // Loaded via Tauri IPC
  let fullDict: any = { globals: [], commands: {} };
  let commandDict: Record<string, string[]> = {};
  let rootCommands: string[] = [];
  
  // Learning context
  let learnType: "global" | "param" | null = null;
  let learnCtx = "";

  // History tracking state
  let isNavigatingHistory = false;
  let pendingHistoryRedrawRepair = false;
  let historyRedrawRepairTimer: ReturnType<typeof setTimeout> | null = null;

  const terminalFontFamily = "'MesloLGS NF', 'Meslo LG S DZ for Powerline', 'Hack Nerd Font Mono', 'CaskaydiaMono Nerd Font', 'Cascadia Mono', 'DejaVu Sans Mono', 'Liberation Mono', monospace";

  function getSelectedTerminalText(): string {
    if (!term) return "";
    return term.getSelection() || "";
  }

  function updateContextMenuSelection() {
    const selectionText = getSelectedTerminalText();
    terminalContextMenu = {
      ...terminalContextMenu,
      selectionText,
      hasSelection: selectionText.trim().length > 0
    };
  }

  function closeTerminalContextMenu() {
    terminalContextMenu = {
      ...terminalContextMenu,
      visible: false
    };
  }

  async function positionTerminalContextMenu() {
    await tick();
    if (!terminalContextMenu.visible || !contextMenuEl) return;

    const rect = contextMenuEl.getBoundingClientRect();
    let nextX = terminalContextMenu.x;
    let nextY = terminalContextMenu.y;

    if (nextX + rect.width > window.innerWidth - 8) {
      nextX = Math.max(8, window.innerWidth - rect.width - 8);
    }
    if (nextY + rect.height > window.innerHeight - 8) {
      nextY = Math.max(8, window.innerHeight - rect.height - 8);
    }

    if (nextX !== terminalContextMenu.x || nextY !== terminalContextMenu.y) {
      terminalContextMenu = {
        ...terminalContextMenu,
        x: nextX,
        y: nextY
      };
    }
  }

  async function copySelection() {
    if (!terminalContextMenu.hasSelection) return;

    try {
      await navigator.clipboard.writeText(terminalContextMenu.selectionText);
      closeTerminalContextMenu();
      term.focus();
    } catch (e) {
      console.error("Failed to copy terminal selection.", e);
    }
  }

  async function pasteClipboard() {
    try {
      const clipboardText = await navigator.clipboard.readText();
      if (clipboardText.length > 0) {
        await invoke("write_stdin", { sessionId, data: clipboardText });
      }
      closeTerminalContextMenu();
      term.focus();
    } catch (e) {
      console.error("Failed to paste clipboard into terminal.", e);
    }
  }

  async function searchSelectionOnWeb() {
    if (!terminalContextMenu.hasSelection) return;

    const query = terminalContextMenu.selectionText.trim();
    if (!query) return;

    try {
      await openUrl(`https://www.google.com/search?q=${encodeURIComponent(query)}`);
      closeTerminalContextMenu();
      term.focus();
    } catch (e) {
      console.error("Failed to open browser search for terminal selection.", e);
    }
  }

  function handleTerminalContextMenu(event: MouseEvent) {
    event.preventDefault();
    updateContextMenuSelection();
    terminalContextMenu = {
      ...terminalContextMenu,
      visible: true,
      x: event.clientX,
      y: event.clientY
    };
    void positionTerminalContextMenu();
  }

  function handleGlobalPointerDown(event: MouseEvent) {
    if (!terminalContextMenu.visible) return;
    const target = event.target as Node | null;
    if (contextMenuEl && target && contextMenuEl.contains(target)) return;
    closeTerminalContextMenu();
  }

  function handleGlobalKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      closeTerminalContextMenu();
    }
  }

  function handleTerminalKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      term?.focus();
    }
  }

  function repairHistoryRedrawArtifacts() {
    if (!term) return;
    try {
      term.clearTextureAtlas();
    } catch (e) {
      console.warn("Failed to clear terminal texture atlas.", e);
    }
    term.refresh(0, term.rows - 1);
    pendingHistoryRedrawRepair = false;
    if (historyRedrawRepairTimer) {
      clearTimeout(historyRedrawRepairTimer);
      historyRedrawRepairTimer = null;
    }
  }

  function markHistoryRedrawRepairNeeded() {
    pendingHistoryRedrawRepair = true;
    if (historyRedrawRepairTimer) {
      clearTimeout(historyRedrawRepairTimer);
    }
    historyRedrawRepairTimer = setTimeout(() => {
      if (pendingHistoryRedrawRepair) {
        requestAnimationFrame(() => repairHistoryRedrawArtifacts());
      }
    }, 120);
  }

  async function learnWord(word: string, type: "global" | "param" | null, ctx: string) {
    if (!type || !word) return;
    if (type === "global") {
      if (!fullDict.globals) fullDict.globals = [];
      if (!fullDict.globals.includes(word)) fullDict.globals.push(word);
    } else if (type === "param") {
      if (!fullDict.commands) fullDict.commands = {};
      if (!fullDict.commands[ctx]) fullDict.commands[ctx] = [];
      if (!fullDict.commands[ctx].includes(word)) fullDict.commands[ctx].push(word);
    }
    
    try {
      await invoke("save_autocomplete", { dict: fullDict });
      commandDict = fullDict.commands || {};
      rootCommands = (fullDict.globals || []).concat(Object.keys(commandDict)).sort();
    } catch (e) {
      console.error("Failed to learn word:", e);
    }
  }

  onMount(async () => {
    // 0. Load settings and Dictionary
    try {
      let settings: any = await invoke("load_settings");
      highlights = settings.highlights || [];
      scrollback = settings.scrollback || 10000;
      showLineNumbers = settings.show_line_numbers || false;
      enableAutocomplete = settings.enable_autocomplete !== false;
    } catch (e) {
      console.error("Failed to load settings in terminal area:", e);
    }
    try {
      let dict: any = await invoke("load_autocomplete");
      fullDict = dict || { globals: [], commands: {} };
      commandDict = fullDict.commands || {};
      rootCommands = (fullDict.globals || []).concat(Object.keys(commandDict)).sort();
    } catch (e) {
      console.error("Failed to load autocomplete dictionary:", e);
    }

    // 1. Initialize xterm.js
    term = new Terminal({
      cursorBlink: true,
      fontFamily: terminalFontFamily,
      fontSize: 14,
      scrollback: scrollback,
      theme: {
        background: '#0d0e12',
        foreground: '#e2e8f0',
        cursor: '#3b82f6',
        black: '#1e2029',
        red: '#ef4444',
        green: '#10b981',
        yellow: '#f59e0b',
        blue: '#3b82f6',
        magenta: '#8b5cf6',
        cyan: '#06b6d4',
        white: '#cbd5e1',
        brightBlack: '#64748b',
        brightRed: '#f87171',
        brightGreen: '#34d399',
        brightYellow: '#fbbf24',
        brightBlue: '#60a5fa',
        brightMagenta: '#a78bfa',
        brightCyan: '#22d3ee',
        brightWhite: '#f8fafc',
      }
    });

    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);

    // Load Unicode 11 addon to correct character widths for Powerline/Nerd Fonts
    try {
      const unicode11Addon = new Unicode11Addon();
      term.loadAddon(unicode11Addon);
      term.unicode.activeVersion = '11';
    } catch(e) {
      console.warn("Unicode11 addon failed.", e);
    }


    term.open(terminalContainer);

    try {
      webglAddon = new WebglAddon();
      term.loadAddon(webglAddon);
    } catch (e) {
      console.warn("WebGL addon failed, falling back to default renderer.", e);
      webglAddon = null;
    }

    document.addEventListener("mousedown", handleGlobalPointerDown);
    document.addEventListener("keydown", handleGlobalKeydown);
    window.addEventListener("blur", closeTerminalContextMenu);
    window.addEventListener("resize", closeTerminalContextMenu);

    if (typeof document !== 'undefined' && 'fonts' in document) {
      document.fonts.ready.then(() => {
        if (!term) return;
        fitAddon.fit();
        term.clearTextureAtlas();
        term.refresh(0, term.rows - 1);
      }).catch((e) => {
        console.warn("Font readiness check failed.", e);
      });
    }
    
    // Intercept function keys and autocomplete selection
    term.attachCustomKeyEventHandler((e) => {
      // Shift+Tab learning intercept
      if (e.type === 'keydown' && e.shiftKey && e.code === 'Tab') {
        if (enableAutocomplete && currentWord.trim().length >= 2) {
          learnWord(currentWord, learnType, learnCtx);
          showAutocomplete = false;
        }
        e.preventDefault();
        return false;
      }

      // Autocomplete interception (only if visible and rendering suggestions)
      if (enableAutocomplete && showAutocomplete && filteredSuggestions.length > 0) {
        if (e.type === 'keydown') {
          if (e.code === 'ArrowDown') {
            selectedAutocompleteIndex = (selectedAutocompleteIndex + 1) % filteredSuggestions.length;
            e.preventDefault();
            return false;
          } else if (e.code === 'ArrowUp') {
            selectedAutocompleteIndex = (selectedAutocompleteIndex - 1 + filteredSuggestions.length) % filteredSuggestions.length;
            e.preventDefault();
            return false;
          } else if (e.code === 'Tab') {
            let chosen = filteredSuggestions[selectedAutocompleteIndex];
            let remainder = chosen.substring(currentWord.length);
            invoke("write_stdin", { sessionId, data: remainder + " " }).catch(console.error);
            currentWord = "";
            showAutocomplete = false;
            e.preventDefault();
            return false;
          } else if (e.code === 'Escape') {
            showAutocomplete = false;
            currentWord = "";
          }
        }
      }

      if (e.code === 'F10' || e.key === 'F10') {
        if (e.type === 'keydown') {
          e.preventDefault();
        }
        return true; 
      }

      // History navigation tracking
      if (e.type === 'keydown') {
        if ((e.code === 'ArrowUp' || e.code === 'ArrowDown') && !showAutocomplete) {
          isNavigatingHistory = true;
          markHistoryRedrawRepairNeeded();
          return true; // Let xterm handle the history navigation
        }
        
        // Reset history navigation state on printable characters or backspace
        // Ignore modifiers and pure navigation keys when resetting
        if (e.key.length === 1 || e.code === 'Backspace' || e.code === 'Space') {
          isNavigatingHistory = false;
        }
      }

      return true;
    });

    fitAddon.fit();

    // 2. Resize handling
    resizeObserver = new ResizeObserver(() => {
      fitAddon.fit();
      if (!isConnecting) {
        invoke("resize_pty", { sessionId, rows: term.rows, cols: term.cols }).catch(e => console.error("Resize error:", e));
      }
      renderLineNumbers();
    });
    resizeObserver.observe(terminalContainer);

    // 3. Use xterm's native onScroll to sync line gutter
    term.onScroll(() => {
      renderLineNumbers();
    });

    // Also listen to onRender for when terminal redraws (covers all cases)
    term.onRender(() => {
      renderLineNumbers();
    });

    term.onWriteParsed(() => {
      if (pendingHistoryRedrawRepair) {
        requestAnimationFrame(() => repairHistoryRedrawArtifacts());
      }
    });

    // 4. User input and Autocomplete tracking
    term.onData(async (data) => {
      try {
        await invoke("write_stdin", { sessionId, data });
      } catch(e) {
        console.error("Write stdin error", e);
      }
    });

    // Extract word matching dynamically off the echoed layout
    term.onCursorMove(() => {
      if (!enableAutocomplete || isNavigatingHistory) {
         showAutocomplete = false;
         return;
      }

      const buf = term.buffer.active;
      const lineObj = buf.getLine(buf.cursorY + buf.baseY);
      if (!lineObj) return;

      const text = lineObj.translateToString(true).substring(0, buf.cursorX);
      
      const rawWords = text.split(/\s+/);
      const words = rawWords.filter(w => w.length > 0);
      if (text.endsWith(" ") && words.length > 0) {
        words.push("");
      }
      
      if (words.length === 0) {
        showAutocomplete = false;
        return;
      }
      
      const lastWord = words[words.length - 1];

      let potentialCommand = "";
      let potentialCommandIndex = -1;
      for (let i = words.length - 1; i >= 0; i--) {
        if (words[i] !== lastWord && rootCommands.includes(words[i])) {
          potentialCommand = words[i];
          potentialCommandIndex = i;
          break;
        }
      }

      let suggestions: string[] = [];
      let isParamCtx = (potentialCommandIndex !== -1 && words.length - 1 - potentialCommandIndex === 1);
      
      if (isParamCtx) {
         if (commandDict[potentialCommand]) {
           suggestions = commandDict[potentialCommand].filter(c => c.startsWith(lastWord) && c !== lastWord).sort();
         }
         learnType = "param";
         learnCtx = potentialCommand;
      } else {
         if (lastWord.length > 0 && (potentialCommandIndex === -1 || words.length - 1 - potentialCommandIndex !== 1)) {
           suggestions = rootCommands.filter(c => c.startsWith(lastWord) && c !== lastWord).sort();
         }
         learnType = "global";
         learnCtx = "";
      }

      let exactMatchFound = false;
      if (isParamCtx) {
         exactMatchFound = commandDict[potentialCommand] && commandDict[potentialCommand].includes(lastWord);
      } else {
         exactMatchFound = rootCommands.includes(lastWord);
      }

      currentWord = lastWord;

      if (suggestions.length > 0) {
        if (!showAutocomplete) selectedAutocompleteIndex = 0;
        showAutocomplete = true;
        filteredSuggestions = suggestions;
        setTimeout(() => {
          const textArea = terminalContainer.querySelector('.xterm-helper-textarea') as HTMLElement;
          if (textArea) {
            const rect = textArea.getBoundingClientRect();
            // Estimate height based on items (approx 34px per item + 10px padding, max 250px)
            const estimatedHeight = Math.min(filteredSuggestions.length * 34 + 10, 250);
            autocompleteX = rect.left;
            
            // If it would cut off the bottom of the screen, render exactly above the cursor
            if (rect.bottom + estimatedHeight > window.innerHeight - 30) {
               autocompleteY = rect.top - estimatedHeight - 10;
            } else {
               autocompleteY = rect.bottom + 4;
            }
          }
        }, 10);
      } else {
        showAutocomplete = false;
      }
    });

    // 5. SSH output listener with write coalescing — registered BEFORE connect
    // to avoid race condition where initial server output (MOTD/prompt) is lost.
    const ansiEscapeRegex = /(\x1b(?:\[[0-9;?]*[A-Za-z]|\][^\x07]*\x07|\[\?[0-9;]*[hl]|\([ABCDEFGHIJKLMNOPQRSTUVWXYZaz01234567@]))/;

    function applyHighlights(text: string): string {
      if (highlights.length === 0) return text;
      let result = text;
      for (const hl of highlights) {
        try {
          const hasSpecial = /[.*+?^${}()|[\]\\]/.test(hl.keyword);
          const pattern = hasSpecial ? `(${hl.keyword})` : `\\b(${hl.keyword})\\b`;
          const regex = new RegExp(pattern, 'g');
          let colorCode = '31';
          switch (hl.color) {
            case 'red': colorCode = '31'; break;
            case 'green': colorCode = '32'; break;
            case 'yellow': colorCode = '33'; break;
            case 'blue': colorCode = '34'; break;
            case 'magenta': colorCode = '35'; break;
            case 'cyan': colorCode = '36'; break;
          }
          result = result.replace(regex, `\x1b[1;${colorCode}m$1\x1b[0m`);
        } catch(e) {}
      }
      return result;
    }

    let writeBuffer = '';
    let flushScheduled = false;

    function flushWriteBuffer() {
      if (writeBuffer.length > 0 && term) {
        const segments = writeBuffer.split(ansiEscapeRegex);
        let output = '';
        for (const seg of segments) {
          if (seg.startsWith('\x1b')) {
            output += seg;
          } else {
            output += applyHighlights(seg);
          }
        }
        term.write(output);
        writeBuffer = '';
      }
      flushScheduled = false;
    }

    unlistenOutput = await listen("ssh-output", (event: any) => {
      const payload = event.payload;
      if (payload.session_id === sessionId) {
        writeBuffer += payload.data;
        if (!flushScheduled) {
          flushScheduled = true;
          requestAnimationFrame(flushWriteBuffer);
        }
      }
    });

    // 6. Connect (listener is already active, no data will be lost)
    try {
      term.writeln(`\x1b[34m[INFO]\x1b[0m Connecting to ${profile.user}@${profile.host}:${profile.port}...`);
      
      await invoke("connect_ssh", {
        sessionId,
        host: profile.host,
        port: Number(profile.port) || 22,
        user: profile.user || "",
        password: profile.password || null,
        privateKey: profile.private_key || null,
        authType: profile.auth_type || null,
        cols: term.cols,
        rows: term.rows
      });
      
      term.writeln(`\x1b[32m[SUCCESS]\x1b[0m Connected.\r\n`);
      isConnecting = false;
      
      // Delay initial resize slightly to let CSS/Flexbox settle entirely
      setTimeout(() => {
        try {
          fitAddon.fit();
          invoke("resize_pty", { sessionId, rows: term.rows, cols: term.cols }).catch(e => console.error(e));
        } catch(e) {}
      }, 50);
      
    } catch(e: any) {
      isConnecting = false;
      connectionError = e;
      term.writeln(`\r\n\x1b[31m[ERROR]\x1b[0m Connection failed: ${e}`);
    }

    // 7. Reactive settings poll for line numbers toggle + highlight changes
    settingsCheckInterval = setInterval(async () => {
      try {
        let s: any = await invoke("load_settings");
        const newVal = s.show_line_numbers || false;
        if (newVal !== showLineNumbers) {
          showLineNumbers = newVal;
          if (showLineNumbers) {
            await tick();
            renderLineNumbers();
          }
        }
        enableAutocomplete = s.enable_autocomplete !== false;
        highlights = s.highlights || [];
      } catch(e) {}
    }, 2000);
  });

  onDestroy(() => {
    document.removeEventListener("mousedown", handleGlobalPointerDown);
    document.removeEventListener("keydown", handleGlobalKeydown);
    window.removeEventListener("blur", closeTerminalContextMenu);
    window.removeEventListener("resize", closeTerminalContextMenu);
    if (resizeObserver) resizeObserver.disconnect();
    if (unlistenOutput) unlistenOutput();
    if (settingsCheckInterval) clearInterval(settingsCheckInterval);
    if (historyRedrawRepairTimer) clearTimeout(historyRedrawRepairTimer);
    if (term) term.dispose();
  });

  function renderLineNumbers() {
    if (!showLineNumbers || !term || !lineNumberEl || !terminalContainer) return;
    const buf = term.buffer.active;
    const viewportY = buf.viewportY;
    const rows = term.rows;

    // Detect actual line height pixel metric dynamically using floating-point rectangles rather than integers
    let dynamicLineHeight = 16.8; // Safe generic fallback for 14px font
    try {
       // Prefer xterm's internal true measurement first to bypass browser rounding entirely
       const coreHeight = (term as any)._core?._renderService?.dimensions?.css?.cell?.height;
       if (coreHeight && coreHeight > 0) {
           dynamicLineHeight = coreHeight;
       } else {
           const firstRowEl = terminalContainer.querySelector('.xterm-rows > div, .xterm-accessibility-tree > div') as HTMLElement;
           if (firstRowEl) {
             const rect = firstRowEl.getBoundingClientRect();
             if (rect.height > 0) dynamicLineHeight = rect.height;
           }
       }
    } catch(e) {}

    let html = '';
    for (let i = 0; i < rows; i++) {
      const lineNum = viewportY + i + 1;
      // Use absolute tracking per-row to completely prevent subpixel browser stacking drifts from compounding
      html += `<div class="ln" style="position: absolute; top: ${i * dynamicLineHeight}px; height: ${dynamicLineHeight}px; width: 100%; right: 0;">${lineNum}</div>`;
    }
    lineNumberEl.style.position = 'relative';
    lineNumberEl.innerHTML = html;
  }
</script>

<div
  class="terminal-outer"
  role="button"
  tabindex="0"
  aria-label="SSH terminal"
  on:click={() => term && term.focus()}
  on:keydown={handleTerminalKeydown}
  on:contextmenu={handleTerminalContextMenu}
>
  {#if showLineNumbers}
    <div class="line-gutter" bind:this={lineNumberEl}></div>
  {/if}
  <div class="terminal-container" bind:this={terminalContainer}></div>

  {#if terminalContextMenu.visible}
    <div
      class="terminal-context-menu"
      bind:this={contextMenuEl}
      style:left={`${terminalContextMenu.x}px`}
      style:top={`${terminalContextMenu.y}px`}
      role="menu"
      aria-label="Terminal context menu"
    >
      <button class="terminal-context-item" type="button" on:click={copySelection} disabled={!terminalContextMenu.hasSelection}>
        Copy
      </button>
      <button class="terminal-context-item" type="button" on:click={pasteClipboard}>
        Paste
      </button>
      <button class="terminal-context-item" type="button" on:click={searchSelectionOnWeb} disabled={!terminalContextMenu.hasSelection}>
        Search on Web
      </button>
    </div>
  {/if}
  
  <AutocompleteOverlay 
    bind:visible={showAutocomplete}
    x={autocompleteX}
    y={autocompleteY}
    suggestions={filteredSuggestions}
    selectedIndex={selectedAutocompleteIndex}
    on:select={(e) => {
      let chosen = e.detail;
      let remainder = chosen.substring(currentWord.length);
      invoke("write_stdin", { sessionId, data: remainder + " " }).catch(console.error);
      currentWord = "";
      showAutocomplete = false;
      term.focus();
    }}
  />
</div>

<style>
  .terminal-outer {
    display: flex;
    position: relative;
    width: 100%;
    height: 100%;
    background-color: var(--bg-darker);
    padding: 0px 4px 6px 4px;
    box-sizing: border-box;
  }

  .line-gutter {
    width: 52px;
    min-width: 52px;
    background-color: #0a0b0f;
    border-right: 1px solid #1e2029;
    font-family: 'MesloLGS NF', 'Meslo LG S DZ for Powerline', 'Hack Nerd Font Mono', 'CaskaydiaMono Nerd Font', 'Cascadia Mono', 'DejaVu Sans Mono', 'Liberation Mono', monospace;
    font-size: 14px;
    color: #4a5568;
    text-align: right;
    user-select: none;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    margin-right: 4px;
  }

  .line-gutter :global(.ln) {
    padding-right: 8px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }

  .terminal-container {
    flex: 1 1 0;
    min-width: 0;
    min-height: 0;
    height: 100%;
    background-color: var(--bg-darker);
    overflow: hidden;
  }

  .terminal-context-menu {
    position: fixed;
    z-index: 1400;
    min-width: 184px;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    background: rgba(10, 12, 18, 0.98);
    border: 1px solid #283244;
    border-radius: 10px;
    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(14px);
  }

  .terminal-context-item {
    display: flex;
    align-items: center;
    width: 100%;
    padding: 10px 12px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: #dbe4f0;
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition: background-color 120ms ease, color 120ms ease;
  }

  .terminal-context-item:hover:not(:disabled) {
    background: #1a2435;
    color: #ffffff;
  }

  .terminal-context-item:disabled {
    color: #64748b;
    cursor: not-allowed;
  }

  :global(.xterm-viewport::-webkit-scrollbar) {
    width: 8px;
  }
  
  :global(.xterm-viewport::-webkit-scrollbar-track) {
    background: transparent;
  }
  
  :global(.xterm-viewport::-webkit-scrollbar-thumb) {
    background-color: #334155;
    border-radius: 4px;
  }
</style>
