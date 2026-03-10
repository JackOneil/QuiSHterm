<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Settings, X } from "lucide-svelte";

  const dispatch = createEventDispatcher();
  let settings: any = {
    debug_mode: false,
    highlights: [],
    folders: [],
    scrollback: 10000,
    show_line_numbers: false,
    enable_autocomplete: true
  };
  let settingsPath = "";
  let modalEl: HTMLElement;
  let resizeObserver: ResizeObserver | null = null;

  onMount(async () => {
    try {
      settings = await invoke("load_settings");
      if (!settings.highlights) settings.highlights = [];
      if (!settings.folders) settings.folders = [];
    } catch(e) {
      console.error(e);
    }
    try {
      settingsPath = await invoke("get_settings_path_info");
    } catch(e) {
      console.error(e);
    }
    // Restore saved modal size
    if (modalEl) {
      const savedW = localStorage.getItem('settings_modal_width');
      const savedH = localStorage.getItem('settings_modal_height');
      if (savedW) modalEl.style.width = savedW;
      if (savedH) modalEl.style.height = savedH;
      // Watch for resize
      resizeObserver = new ResizeObserver((entries) => {
        for (const entry of entries) {
          localStorage.setItem('settings_modal_width', entry.target.clientWidth + 'px');
          localStorage.setItem('settings_modal_height', entry.target.clientHeight + 'px');
        }
      });
      resizeObserver.observe(modalEl);
    }
  });

  onDestroy(() => {
    if (resizeObserver) resizeObserver.disconnect();
  });

  let backdropMouseDown = false;

  function close() {
    dispatch("close");
  }

  function onBackdropMouseDown(e: MouseEvent) {
    // Only mark if the click target is the backdrop itself
    if (e.target === e.currentTarget) {
      backdropMouseDown = true;
    }
  }

  function onBackdropMouseUp(e: MouseEvent) {
    if (backdropMouseDown && e.target === e.currentTarget) {
      close();
    }
    backdropMouseDown = false;
  }

  async function saveSettings() {
    try {
      await invoke("save_settings", { settings });
      close();
    } catch(e) {
      console.error(e);
    }
  }

  let activeTab = "Dictionary";
  let newKeyword = "";
  let newColor = "red";

  function addHighlight() {
    if (newKeyword.trim() !== "") {
      settings.highlights = [...settings.highlights, { keyword: newKeyword.trim(), color: newColor }];
      newKeyword = "";
    }
  }

  function removeHighlight(index: number) {
    settings.highlights = settings.highlights.filter((_: any, i: number) => i !== index);
  }
</script>

<div class="modal-backdrop" on:mousedown={onBackdropMouseDown} on:mouseup={onBackdropMouseUp} aria-hidden="true">
  <div class="modal" on:click|stopPropagation on:keydown|stopPropagation role="dialog" aria-label="Settings" tabindex="-1" bind:this={modalEl}>
    <div class="modal-header">
      <h2><Settings size={18} class="mr-2"/> Settings</h2>
      <button class="icon-btn" on:click={close}><X size={18} /></button>
    </div>

    <div class="modal-body">
      <div class="sidebar">
        <button class="menu-item {activeTab === 'General' ? 'active' : ''}" on:click={() => activeTab = 'General'}>
          General
        </button>
        <button class="menu-item {activeTab === 'Terminal' ? 'active' : ''}" on:click={() => activeTab = 'Terminal'}>
          Terminal
        </button>
        <button class="menu-item {activeTab === 'Dictionary' ? 'active' : ''}" on:click={() => activeTab = 'Dictionary'}>
          Dictionary
        </button>
      </div>

      <div class="content">
        {#if activeTab === 'General'}
          <div class="settings-group">
            <h3>Logging</h3>
            <label class="toggle-row">
              <span class="label-text">
                <strong>Enable Debug Mode</strong>
                <span class="desc">Writes extensive SSH handshake and connection logs to a .log file next to the executable.</span>
              </span>
              <input type="checkbox" bind:checked={settings.debug_mode} />
            </label>
          </div>
          <div class="settings-group mt-3">
            <h3>Storage</h3>
            <div class="info-row">
              <span class="label-text">
                <strong>Settings file location</strong>
                <span class="desc path-text">{settingsPath || 'Loading...'}</span>
              </span>
            </div>
          </div>
        {:else if activeTab === 'Terminal'}
          <div class="settings-group">
            <h3>Autocomplete</h3>
            <label class="toggle-row">
              <span class="label-text">
                <strong>Enable Smart Autocomplete</strong>
                <span class="desc">Shows contextual floating dictionary popups while typing Linux/Bash commands.</span>
              </span>
              <input type="checkbox" bind:checked={settings.enable_autocomplete} />
            </label>
          </div>
          <div class="settings-group mt-3">
            <h3>Scrollback Buffer</h3>
            <label class="toggle-row">
              <span class="label-text">
                <strong>History lines</strong>
                <span class="desc">Number of lines to keep in terminal scrollback buffer. Higher = more memory usage. Requires reconnect.</span>
              </span>
              <input type="number" class="num-input" bind:value={settings.scrollback} min="100" max="100000" step="1000" />
            </label>
          </div>
          <div class="settings-group mt-3">
            <h3>Line Numbers</h3>
            <label class="toggle-row">
              <span class="label-text">
                <strong>Show line numbers</strong>
                <span class="desc">Display a line number gutter on the left side of the terminal for easier code reference. Requires reconnect.</span>
              </span>
              <input type="checkbox" bind:checked={settings.show_line_numbers} />
            </label>
          </div>
        {:else if activeTab === 'Dictionary'}
          <div class="settings-group dict-group">
            <h3>Color Highlighting Dictionary</h3>
            <p class="desc mb-2">Configure words to automatically highlight in the terminal output.</p>
            
            <div class="form-row mb-3">
              <input type="text" placeholder="Keyword" bind:value={newKeyword} on:keydown={(e) => e.key === 'Enter' && addHighlight()} />
              <select bind:value={newColor}>
                <option value="red">Red</option>
                <option value="green">Green</option>
                <option value="yellow">Yellow</option>
                <option value="blue">Blue</option>
                <option value="magenta">Magenta</option>
                <option value="cyan">Cyan</option>
              </select>
              <button class="secondary-btn" on:click={addHighlight}>Add</button>
            </div>

            <div class="highlight-list">
              {#each settings.highlights as hl, i}
                <div class="highlight-item">
                  <span class="hl-keyword">{hl.keyword}</span>
                  <div class="hl-controls">
                    <span class="badge color-{hl.color}">{hl.color}</span>
                    <button class="icon-btn text-danger ml-1" on:click={() => removeHighlight(i)} title="Remove">
                      <X size={14}/>
                    </button>
                  </div>
                </div>
              {/each}
              {#if settings.highlights.length === 0}
                 <div class="empty-state">No highlights configured.</div>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    </div>

    <div class="modal-footer">
      <button class="secondary-btn" on:click={close}>Cancel</button>
      <button class="primary-btn" on:click={saveSettings}>Save Changes</button>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0; left: 0; right: 0; bottom: 0;
    background-color: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background-color: var(--bg-surface);
    width: 650px;
    min-width: 400px;
    min-height: 350px;
    max-width: 90vw;
    max-height: 85vh;
    border-radius: 12px;
    border: 1px solid var(--border);
    box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    resize: both;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  .modal-header h2 {
    margin: 0; font-size: 18px; font-weight: 500;
    display: flex; align-items: center;
  }

  .modal-body {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .sidebar {
    width: 150px;
    flex-shrink: 0;
    background-color: var(--bg-dark);
    border-right: 1px solid var(--border);
    padding: 12px 0;
  }

  .menu-item {
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    padding: 10px 20px;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 14px;
  }

  .menu-item:hover {
    color: var(--text-main);
  }

  .menu-item.active {
    background-color: var(--bg-surface);
    color: var(--text-main);
    border-left: 3px solid var(--accent);
  }

  .content {
    flex: 1;
    padding: 20px;
    overflow-y: auto;
  }

  .dict-group {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .settings-group h3 {
    margin-top: 0;
    margin-bottom: 16px;
    font-size: 15px;
    color: var(--text-main);
    font-weight: 500;
  }

  .toggle-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px;
    background-color: var(--bg-dark);
    border: 1px solid var(--border);
    border-radius: 8px;
    cursor: pointer;
  }

  .num-input {
    width: 90px;
    padding: 6px 10px;
    background: var(--bg-darker, #0d0e12);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: white;
    font-size: 14px;
    text-align: right;
    outline: none;
  }
  .num-input:focus { border-color: var(--accent); }

  .info-row {
    padding: 12px;
    background-color: var(--bg-dark);
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  .label-text {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .label-text strong {
    font-weight: 500;
    font-size: 14px;
  }

  .label-text .desc {
    font-size: 12px;
    color: var(--text-muted);
  }

  .path-text {
    font-family: monospace;
    word-break: break-all;
    user-select: all;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    padding: 16px 20px;
    border-top: 1px solid var(--border);
    background-color: var(--bg-dark);
  }

  .icon-btn {
    background: none; border: none; color: var(--text-muted);
    cursor: pointer; padding: 6px; border-radius: 6px;
    display: flex; align-items: center;
  }
  .icon-btn:hover { background-color: var(--bg-hover); color: var(--text-main); }

  .primary-btn {
    background-color: var(--accent); color: white; border: none;
    padding: 8px 16px; border-radius: 6px; font-weight: 500; cursor: pointer;
  }
  .primary-btn:hover { background-color: var(--accent-hover); }
  
  .secondary-btn {
    background-color: transparent; color: var(--text-main);
    border: 1px solid var(--border); padding: 8px 16px;
    border-radius: 6px; cursor: pointer;
  }
  .secondary-btn:hover { background-color: var(--bg-hover); }

  .form-row { display: flex; gap: 8px; }
  .form-row input[type="text"] { flex: 1; padding: 8px 12px; background: var(--bg-dark); border: 1px solid var(--border); border-radius: 6px; color: white; }
  .form-row input[type="text"]:focus { outline: none; border-color: var(--accent); }
  .form-row select { padding: 8px 12px; background: var(--bg-dark); border: 1px solid var(--border); border-radius: 6px; color: white; outline: none; cursor: pointer; }

  .highlight-list { display: flex; flex-direction: column; gap: 6px; flex: 1; min-height: 0; overflow-y: auto; padding-right: 4px; }
  .highlight-item { display: flex; justify-content: space-between; align-items: center; background: var(--bg-dark); border: 1px solid var(--border); border-radius: 6px; padding: 6px 12px; }
  .hl-keyword { font-family: monospace; font-size: 13px; font-weight: 500;}
  .hl-controls { display: flex; align-items: center; gap: 8px;}
  .highlight-item .badge { font-size: 11px; padding: 2px 8px; border-radius: 4px; color: white; font-weight: bold; text-transform: uppercase;}
  .color-red { background-color: #ef4444; }
  .color-green { background-color: #10b981; }
  .color-yellow { background-color: #f59e0b; }
  .color-blue { background-color: #3b82f6; }
  .color-magenta { background-color: #8b5cf6; }
  .color-cyan { background-color: #06b6d4; }
  
  .empty-state { text-align: center; color: var(--text-muted); padding: 20px; font-size: 13px; font-style: italic;}

  .text-danger { color: #ef4444; }
  .text-danger:hover { color: #fca5a5; background-color: rgba(239, 68, 68, 0.1); }

  :global(.mr-2) { margin-right: 8px; }
  :global(.mr-1) { margin-right: 4px; }
  :global(.ml-1) { margin-left: 4px; }
  :global(.mb-2) { margin-bottom: 8px; }
  :global(.mb-3) { margin-bottom: 12px; }
  :global(.mt-3) { margin-top: 12px; }
</style>
