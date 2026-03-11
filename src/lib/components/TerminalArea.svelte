<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { Eye, EyeOff, KeyRound } from "lucide-svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { WebglAddon } from "@xterm/addon-webgl";
  import { Unicode11Addon } from "@xterm/addon-unicode11";
  import AutocompleteOverlay from "./AutocompleteOverlay.svelte";
  import "@xterm/xterm/css/xterm.css";

  export let sessionId: string;
  export let profile: any;
  export let layoutVersion = 0;

  let terminalOuter: HTMLElement;
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
  let fitFrame: number | null = null;
  let passwordPromptInput: HTMLInputElement;
  let connectionPassword = profile?.password || "";
  let showPasswordPrompt = false;
  let passwordPromptValue = "";
  let passwordPromptError = "";
  let revealPromptPassword = false;

  let terminalContextMenu = {
    visible: false,
    x: 0,
    y: 0,
    hasSelection: false,
    selectionText: ""
  };

  let currentWord = "";
  let showAutocomplete = false;
  let autocompleteX = 0;
  let autocompleteY = 0;
  let filteredSuggestions: string[] = [];
  let selectedAutocompleteIndex = 0;

  let fullDict: any = { globals: [], commands: {} };
  let commandDict: Record<string, string[]> = {};
  let rootCommands: string[] = [];

  let learnType: "global" | "param" | null = null;
  let learnCtx = "";

  let isNavigatingHistory = false;
  let pendingHistoryRedrawRepair = false;
  let historyRedrawRepairTimer: ReturnType<typeof setTimeout> | null = null;
  let lastLayoutVersion = -1;

  const terminalFontFamily = "'MesloLGS NF', 'Meslo LG S DZ for Powerline', 'Hack Nerd Font Mono', 'CaskaydiaMono Nerd Font', 'Cascadia Mono', 'DejaVu Sans Mono', 'Liberation Mono', monospace";

  function getConnectionErrorMessage(error: unknown): string {
    if (typeof error === "string") return error;
    if (error instanceof Error) return error.message;
    return String(error ?? "Unknown connection error");
  }

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
    if (showPasswordPrompt) {
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      term?.focus();
    }
  }

  function focusTerminal() {
    if (showPasswordPrompt) {
      return;
    }

    term?.focus();
  }

  async function focusPasswordPromptInput() {
    await tick();
    passwordPromptInput?.focus();
    passwordPromptInput?.select();
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

  function refreshTerminalViewport(forcePtyResize = false) {
    if (!term || !fitAddon) return;

    if (fitFrame !== null) {
      cancelAnimationFrame(fitFrame);
    }

    fitFrame = requestAnimationFrame(() => {
      fitFrame = null;

      try {
        fitAddon.fit();
      } catch (e) {
        console.warn("Terminal fit failed.", e);
        return;
      }

      try {
        term.clearTextureAtlas();
      } catch (e) {
        console.warn("Failed to clear terminal texture atlas during viewport refresh.", e);
      }

      term.refresh(0, Math.max(term.rows - 1, 0));
      renderLineNumbers();

      if (forcePtyResize && !isConnecting) {
        invoke("resize_pty", { sessionId, rows: term.rows, cols: term.cols }).catch((e) => console.error("Resize error:", e));
      }
    });
  }

  $: if (term && layoutVersion !== lastLayoutVersion) {
    lastLayoutVersion = layoutVersion;
    setTimeout(() => refreshTerminalViewport(true), 0);
    setTimeout(() => refreshTerminalViewport(true), 80);
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

  async function connectSession(passwordOverride: string | null = null) {
    try {
      isConnecting = true;
      connectionError = "";
      const requestedTerminalType = profile.terminal_type || "xterm-256color";

      term.writeln(`\x1b[34m[INFO]\x1b[0m Connecting to ${profile.user}@${profile.host}:${profile.port}...`);
      term.writeln(`\x1b[34m[INFO]\x1b[0m Requested PTY terminal type: ${requestedTerminalType}`);

      const passwordToUse = passwordOverride ?? connectionPassword;
      await invoke("connect_ssh", {
        sessionId,
        host: profile.host,
        port: Number(profile.port) || 22,
        user: profile.user || "",
        password: passwordToUse?.trim() ? passwordToUse : null,
        privateKey: profile.private_key || null,
        authType: profile.auth_type || null,
        terminalType: requestedTerminalType,
        cols: term.cols,
        rows: term.rows
      });

      showPasswordPrompt = false;
      passwordPromptError = "";
      term.writeln(`\x1b[32m[SUCCESS]\x1b[0m Connected.\r\n`);
      isConnecting = false;

      setTimeout(() => {
        refreshTerminalViewport(true);
      }, 50);
    } catch (error: unknown) {
      const message = getConnectionErrorMessage(error);

      if (message === "PASSWORD_REQUIRED") {
        isConnecting = false;
        showPasswordPrompt = true;
        passwordPromptValue = "";
        passwordPromptError = "";
        revealPromptPassword = false;
        term.writeln(`\r\n\x1b[33m[AUTH]\x1b[0m Password required for ${profile.user}@${profile.host}.`);
        void focusPasswordPromptInput();
        return;
      }

      if (message.startsWith("PASSWORD_AUTH_FAILED:")) {
        isConnecting = false;
        showPasswordPrompt = true;
        passwordPromptError = message.replace("PASSWORD_AUTH_FAILED:", "").trim() || "Password authentication failed.";
        term.writeln(`\r\n\x1b[31m[ERROR]\x1b[0m Password authentication failed.`);
        void focusPasswordPromptInput();
        return;
      }

      isConnecting = false;
      connectionError = message;
      term.writeln(`\r\n\x1b[31m[ERROR]\x1b[0m Connection failed: ${message}`);
    }
  }

  async function submitPasswordPrompt() {
    connectionPassword = passwordPromptValue;
    passwordPromptError = "";
    showPasswordPrompt = false;
    await connectSession(passwordPromptValue);
  }

  function cancelPasswordPrompt() {
    showPasswordPrompt = false;
    passwordPromptError = "";
    connectionError = "Password prompt cancelled.";
    term.writeln(`\r\n\x1b[33m[INFO]\x1b[0m Password prompt cancelled.`);
  }

  onMount(async () => {
    try {
      const settings: any = await invoke("load_settings");
      highlights = settings.highlights || [];
      scrollback = settings.scrollback || 10000;
      showLineNumbers = settings.show_line_numbers || false;
      enableAutocomplete = settings.enable_autocomplete !== false;
    } catch (e) {
      console.error("Failed to load settings in terminal area:", e);
    }
    try {
      const dict: any = await invoke("load_autocomplete");
      fullDict = dict || { globals: [], commands: {} };
      commandDict = fullDict.commands || {};
      rootCommands = (fullDict.globals || []).concat(Object.keys(commandDict)).sort();
    } catch (e) {
      console.error("Failed to load autocomplete dictionary:", e);
    }

    term = new Terminal({
      cursorBlink: true,
      fontFamily: terminalFontFamily,
      fontSize: 14,
      scrollback,
      theme: {
        background: "#0d0e12",
        foreground: "#e2e8f0",
        cursor: "#3b82f6",
        black: "#1e2029",
        red: "#ef4444",
        green: "#10b981",
        yellow: "#f59e0b",
        blue: "#3b82f6",
        magenta: "#8b5cf6",
        cyan: "#06b6d4",
        white: "#cbd5e1",
        brightBlack: "#64748b",
        brightRed: "#f87171",
        brightGreen: "#34d399",
        brightYellow: "#fbbf24",
        brightBlue: "#60a5fa",
        brightMagenta: "#a78bfa",
        brightCyan: "#22d3ee",
        brightWhite: "#f8fafc",
      }
    });

    fitAddon = new FitAddon();
    term.loadAddon(fitAddon);

    try {
      const unicode11Addon = new Unicode11Addon();
      term.loadAddon(unicode11Addon);
      term.unicode.activeVersion = "11";
    } catch (e) {
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

    if (typeof document !== "undefined" && "fonts" in document) {
      document.fonts.ready.then(() => {
        if (!term) return;
        refreshTerminalViewport(false);
      }).catch((e) => {
        console.warn("Font readiness check failed.", e);
      });
    }

    term.attachCustomKeyEventHandler((e) => {
      if (e.type === "keydown" && e.shiftKey && e.code === "Tab") {
        if (enableAutocomplete && currentWord.trim().length >= 2) {
          learnWord(currentWord, learnType, learnCtx);
          showAutocomplete = false;
        }
        e.preventDefault();
        return false;
      }

      if (enableAutocomplete && showAutocomplete && filteredSuggestions.length > 0) {
        if (e.type === "keydown") {
          if (e.code === "ArrowDown") {
            selectedAutocompleteIndex = (selectedAutocompleteIndex + 1) % filteredSuggestions.length;
            e.preventDefault();
            return false;
          }
          if (e.code === "ArrowUp") {
            selectedAutocompleteIndex = (selectedAutocompleteIndex - 1 + filteredSuggestions.length) % filteredSuggestions.length;
            e.preventDefault();
            return false;
          }
          if (e.code === "Tab") {
            const chosen = filteredSuggestions[selectedAutocompleteIndex];
            const remainder = chosen.substring(currentWord.length);
            invoke("write_stdin", { sessionId, data: remainder + " " }).catch(console.error);
            currentWord = "";
            showAutocomplete = false;
            e.preventDefault();
            return false;
          }
          if (e.code === "Escape") {
            showAutocomplete = false;
            currentWord = "";
          }
        }
      }

      if (e.code === "F10" || e.key === "F10") {
        if (e.type === "keydown") {
          e.preventDefault();
        }
        return true;
      }

      if (e.type === "keydown") {
        if ((e.code === "ArrowUp" || e.code === "ArrowDown") && !showAutocomplete) {
          isNavigatingHistory = true;
          markHistoryRedrawRepairNeeded();
          return true;
        }

        if (e.key.length === 1 || e.code === "Backspace" || e.code === "Space") {
          isNavigatingHistory = false;
        }
      }

      return true;
    });

    refreshTerminalViewport(false);

    resizeObserver = new ResizeObserver(() => {
      refreshTerminalViewport(true);
    });
    resizeObserver.observe(terminalOuter);
    resizeObserver.observe(terminalContainer);

    term.onScroll(() => {
      renderLineNumbers();
    });

    term.onRender(() => {
      renderLineNumbers();
    });

    term.onWriteParsed(() => {
      if (pendingHistoryRedrawRepair) {
        requestAnimationFrame(() => repairHistoryRedrawArtifacts());
      }
    });

    term.onData(async (data) => {
      try {
        await invoke("write_stdin", { sessionId, data });
      } catch (e) {
        console.error("Write stdin error", e);
      }
    });

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
      const words = rawWords.filter((word) => word.length > 0);
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
      for (let index = words.length - 1; index >= 0; index -= 1) {
        if (words[index] !== lastWord && rootCommands.includes(words[index])) {
          potentialCommand = words[index];
          potentialCommandIndex = index;
          break;
        }
      }

      let suggestions: string[] = [];
      const isParamCtx = potentialCommandIndex !== -1 && words.length - 1 - potentialCommandIndex === 1;

      if (isParamCtx) {
        if (commandDict[potentialCommand]) {
          suggestions = commandDict[potentialCommand].filter((candidate) => candidate.startsWith(lastWord) && candidate !== lastWord).sort();
        }
        learnType = "param";
        learnCtx = potentialCommand;
      } else {
        if (lastWord.length > 0 && (potentialCommandIndex === -1 || words.length - 1 - potentialCommandIndex !== 1)) {
          suggestions = rootCommands.filter((candidate) => candidate.startsWith(lastWord) && candidate !== lastWord).sort();
        }
        learnType = "global";
        learnCtx = "";
      }

      currentWord = lastWord;

      if (suggestions.length > 0) {
        if (!showAutocomplete) selectedAutocompleteIndex = 0;
        showAutocomplete = true;
        filteredSuggestions = suggestions;
        setTimeout(() => {
          const textArea = terminalContainer.querySelector(".xterm-helper-textarea") as HTMLElement;
          if (textArea) {
            const rect = textArea.getBoundingClientRect();
            const estimatedHeight = Math.min(filteredSuggestions.length * 34 + 10, 250);
            autocompleteX = rect.left;

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

    const ansiEscapeRegex = /(\x1b(?:\[[0-9;?]*[A-Za-z]|\][^\x07]*\x07|\[\?[0-9;]*[hl]|\([ABCDEFGHIJKLMNOPQRSTUVWXYZaz01234567@]))/;

    function applyHighlights(text: string): string {
      if (highlights.length === 0) return text;
      let result = text;
      for (const highlight of highlights) {
        try {
          const hasSpecial = /[.*+?^${}()|[\]\\]/.test(highlight.keyword);
          const pattern = hasSpecial ? `(${highlight.keyword})` : `\\b(${highlight.keyword})\\b`;
          const regex = new RegExp(pattern, "g");
          let colorCode = "31";
          switch (highlight.color) {
            case "red": colorCode = "31"; break;
            case "green": colorCode = "32"; break;
            case "yellow": colorCode = "33"; break;
            case "blue": colorCode = "34"; break;
            case "magenta": colorCode = "35"; break;
            case "cyan": colorCode = "36"; break;
          }
          result = result.replace(regex, `\x1b[1;${colorCode}m$1\x1b[0m`);
        } catch (e) {}
      }
      return result;
    }

    let writeBuffer = "";
    let flushScheduled = false;

    function flushWriteBuffer() {
      if (writeBuffer.length > 0 && term) {
        const segments = writeBuffer.split(ansiEscapeRegex);
        let output = "";
        for (const segment of segments) {
          if (segment.startsWith("\x1b")) {
            output += segment;
          } else {
            output += applyHighlights(segment);
          }
        }
        term.write(output);
        writeBuffer = "";
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

    await connectSession();

    settingsCheckInterval = setInterval(async () => {
      try {
        const settings: any = await invoke("load_settings");
        const newLineNumbersValue = settings.show_line_numbers || false;
        if (newLineNumbersValue !== showLineNumbers) {
          showLineNumbers = newLineNumbersValue;
          if (showLineNumbers) {
            await tick();
            renderLineNumbers();
          }
        }
        enableAutocomplete = settings.enable_autocomplete !== false;
        highlights = settings.highlights || [];
      } catch (e) {}
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
    if (fitFrame !== null) cancelAnimationFrame(fitFrame);
    if (term) term.dispose();
  });

  function renderLineNumbers() {
    if (!showLineNumbers || !term || !lineNumberEl || !terminalContainer) return;
    const buf = term.buffer.active;
    const viewportY = buf.viewportY;
    const rows = term.rows;

    let dynamicLineHeight = 16.8;
    try {
      const coreHeight = (term as any)._core?._renderService?.dimensions?.css?.cell?.height;
      if (coreHeight && coreHeight > 0) {
        dynamicLineHeight = coreHeight;
      } else {
        const firstRowEl = terminalContainer.querySelector(".xterm-rows > div, .xterm-accessibility-tree > div") as HTMLElement;
        if (firstRowEl) {
          const rect = firstRowEl.getBoundingClientRect();
          if (rect.height > 0) dynamicLineHeight = rect.height;
        }
      }
    } catch (e) {}

    let html = "";
    for (let index = 0; index < rows; index += 1) {
      const lineNum = viewportY + index + 1;
      html += `<div class="ln" style="position: absolute; top: ${index * dynamicLineHeight}px; height: ${dynamicLineHeight}px; width: 100%; right: 0;">${lineNum}</div>`;
    }
    lineNumberEl.style.position = "relative";
    lineNumberEl.innerHTML = html;
  }
</script>

<div
  class="terminal-outer"
  bind:this={terminalOuter}
  role="button"
  tabindex="0"
  aria-label="SSH terminal"
  on:click={focusTerminal}
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

  {#if showPasswordPrompt}
    <div class="password-prompt-backdrop" role="presentation" on:mousedown|stopPropagation on:click|stopPropagation>
      <form class="password-prompt" on:submit|preventDefault={submitPasswordPrompt} on:mousedown|stopPropagation on:click|stopPropagation>
        <div class="password-prompt-header">
          <div class="password-prompt-title">
            <KeyRound size={16} />
            <span>Password Required</span>
          </div>
        </div>
        <p class="password-prompt-copy">Agent/Pageant and key-based authentication were not accepted for {profile.user}@{profile.host}. Enter the account password to continue.</p>
        <label class="password-prompt-label" for={`password-${sessionId}`}>Password</label>
        <div class="password-prompt-input-row">
          <input
            id={`password-${sessionId}`}
            class="password-prompt-input"
            bind:this={passwordPromptInput}
            type={revealPromptPassword ? "text" : "password"}
            bind:value={passwordPromptValue}
            autocomplete="current-password"
            on:mousedown|stopPropagation
            on:click|stopPropagation
          />
          <button class="password-visibility-btn" type="button" aria-label={revealPromptPassword ? 'Hide password' : 'Show password'} on:click={() => revealPromptPassword = !revealPromptPassword}>
            {#if revealPromptPassword}
              <EyeOff size={16} />
            {:else}
              <Eye size={16} />
            {/if}
          </button>
        </div>
        {#if passwordPromptError}
          <div class="password-prompt-error">{passwordPromptError}</div>
        {/if}
        <div class="password-prompt-actions">
          <button class="terminal-context-item prompt-secondary" type="button" on:click={cancelPasswordPrompt}>Cancel</button>
          <button class="terminal-context-item prompt-primary" type="submit">Connect</button>
        </div>
      </form>
    </div>
  {/if}

  <AutocompleteOverlay
    bind:visible={showAutocomplete}
    x={autocompleteX}
    y={autocompleteY}
    suggestions={filteredSuggestions}
    selectedIndex={selectedAutocompleteIndex}
    on:select={(event) => {
      const chosen = event.detail;
      const remainder = chosen.substring(currentWord.length);
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
    min-width: 0;
    min-height: 0;
    overflow: hidden;
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

  .password-prompt-backdrop {
    position: absolute;
    inset: 0;
    z-index: 1450;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(6, 8, 12, 0.68);
    backdrop-filter: blur(6px);
  }

  .password-prompt {
    width: min(420px, calc(100% - 32px));
    padding: 18px;
    border-radius: 14px;
    border: 1px solid #283244;
    background: #11151d;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.45);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .password-prompt-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .password-prompt-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    color: #f8fafc;
  }

  .password-prompt-copy {
    margin: 0;
    color: #94a3b8;
    font-size: 13px;
    line-height: 1.45;
  }

  .password-prompt-label {
    font-size: 12px;
    color: #cbd5e1;
  }

  .password-prompt-input-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
    align-items: center;
  }

  .password-prompt-input {
    width: 100%;
    box-sizing: border-box;
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid #334155;
    background: #0d0e12;
    color: #f8fafc;
    font: inherit;
    outline: none;
  }

  .password-prompt-input:focus {
    border-color: #3b82f6;
  }

  .password-visibility-btn {
    width: 40px;
    height: 40px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    border: 1px solid #334155;
    background: #0d0e12;
    color: #dbe4f0;
    cursor: pointer;
  }

  .password-visibility-btn:hover {
    background: #172033;
  }

  .password-prompt-error {
    font-size: 12px;
    color: #fca5a5;
  }

  .password-prompt-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .prompt-secondary,
  .prompt-primary {
    width: auto;
    min-width: 100px;
    justify-content: center;
  }

  .prompt-secondary {
    border: 1px solid #334155;
  }

  .prompt-primary {
    background: #2563eb;
  }

  .prompt-primary:hover {
    background: #1d4ed8;
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