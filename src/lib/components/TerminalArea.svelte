<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { WebglAddon } from "@xterm/addon-webgl";
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
  let lineNumberEl: HTMLElement;
  let settingsCheckInterval: ReturnType<typeof setInterval> | null = null;

  onMount(async () => {
    // 0. Load settings
    try {
      let settings: any = await invoke("load_settings");
      highlights = settings.highlights || [];
      scrollback = settings.scrollback || 10000;
      showLineNumbers = settings.show_line_numbers || false;
    } catch (e) {
      console.error("Failed to load settings in terminal area:", e);
    }

    // 1. Initialize xterm.js
    term = new Terminal({
      cursorBlink: true,
      fontFamily: "'Fira Code', 'Cascadia Code', monospace",
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
    term.open(terminalContainer);
    
    // Intercept function keys (especially F10 used by MC/htop) so the browser doesn't swallow them
    term.attachCustomKeyEventHandler((e) => {
      if (e.code === 'F10' || e.key === 'F10') {
        if (e.type === 'keydown') {
          e.preventDefault();
        }
        return true; // Let xterm process it and send it to PTY
      }
      return true;
    });

    try {
        webglAddon = new WebglAddon();
        term.loadAddon(webglAddon);
    } catch(e) {
        console.warn("WebGL addon failed.", e);
    }

    fitAddon.fit();

    // 2. Resize handling
    const resizeObserver = new ResizeObserver(() => {
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

    // 4. User input
    term.onData(async (data) => {
      try {
        await invoke("write_stdin", { sessionId, data });
      } catch(e) {
        console.error("Write stdin error", e);
      }
    });

    // 5. Connect
    try {
      term.writeln(`\x1b[34m[INFO]\x1b[0m Connecting to ${profile.user}@${profile.host}:${profile.port}...`);
      
      await invoke("connect_ssh", {
        sessionId,
        host: profile.host,
        port: Number(profile.port) || 22,
        user: profile.user || "",
        password: profile.password || null,
        privateKey: profile.private_key || null,
        authType: profile.auth_type || null
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

    // 6. SSH output listener
    unlistenOutput = await listen("ssh-output", (event: any) => {
      const payload = event.payload;
      if (payload.session_id === sessionId) {
        let parsedData = payload.data;

        if (highlights.length > 0) {
            highlights.forEach((hl) => {
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
                 parsedData = parsedData.replace(regex, `\x1b[1;${colorCode}m$1\x1b[0m`);
               } catch(e) {}
            });
        }
            
        term.write(parsedData);
      }
    });

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
        highlights = s.highlights || [];
      } catch(e) {}
    }, 2000);

    return () => {
      resizeObserver.disconnect();
    };
  });

  onDestroy(() => {
    if (unlistenOutput) unlistenOutput();
    if (settingsCheckInterval) clearInterval(settingsCheckInterval);
    if (term) term.dispose();
  });

  function renderLineNumbers() {
    if (!showLineNumbers || !term || !lineNumberEl) return;
    const buf = term.buffer.active;
    const baseRow = buf.baseY;
    const cursorRow = buf.cursorY;
    const viewportY = buf.viewportY;
    const rows = term.rows;

    let html = '';
    for (let i = 0; i < rows; i++) {
      const lineNum = viewportY + i + 1;
      html += `<div class="ln">${lineNum}</div>`;
    }
    lineNumberEl.innerHTML = html;
  }
</script>

<div class="terminal-outer" on:click={() => term && term.focus()}>
  {#if showLineNumbers}
    <div class="line-gutter" bind:this={lineNumberEl}></div>
  {/if}
  <div class="terminal-container" bind:this={terminalContainer}></div>
</div>

<style>
  .terminal-outer {
    display: flex;
    width: 100%;
    height: 100%;
    background-color: var(--bg-darker);
  }

  .line-gutter {
    width: 52px;
    min-width: 52px;
    background-color: #0a0b0f;
    border-right: 1px solid #1e2029;
    font-family: 'Fira Code', 'Cascadia Code', monospace;
    font-size: 14px;
    color: #4a5568;
    text-align: right;
    user-select: none;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .line-gutter :global(.ln) {
    padding-right: 8px;
    /* Match xterm's default line height exactly */
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }

  .terminal-container {
    flex: 1 1 0;
    min-width: 0;
    min-height: 0;
    height: 100%;
    padding: 4px;
    background-color: var(--bg-darker);
    overflow: hidden;
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
