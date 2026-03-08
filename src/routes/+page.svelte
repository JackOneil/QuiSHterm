<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { Terminal as TerminalIcon, Settings, Plus, X, PanelRight } from "lucide-svelte";
  import TerminalArea from "$lib/components/TerminalArea.svelte";
  import ConnectionManager from "$lib/components/ConnectionManager.svelte";
  import SettingsManager from "$lib/components/SettingsManager.svelte";
  import QuickConnect from "$lib/components/QuickConnect.svelte";
  import SplitView from "$lib/components/SplitView.svelte";

  let sidebarRef: QuickConnect;

  let showManager = false;
  let showSettings = false;
  let showSidebar = true;
  let activeTabId = "";
  
  export type SplitDirection = 'row' | 'col';
  export interface SplitNode {
    id: string;
    type: 'terminal' | 'split';
    direction?: SplitDirection;
    children?: SplitNode[];
    profile?: any;
  }

  interface PaneLeaf {
    id: string;
    profile: any;
    x: number;
    y: number;
    w: number;
    h: number;
  }

  function getLeaves(node: SplitNode, x: number, y: number, w: number, h: number): PaneLeaf[] {
    if (node.type === 'terminal' || !node.children || node.children.length === 0) {
      return [{ id: node.id, profile: node.profile, x, y, w, h }];
    }
    let leaves: PaneLeaf[] = [];
    if (node.direction === 'row') {
      let childW = w / node.children.length;
      for (let i = 0; i < node.children.length; i++) {
        leaves = leaves.concat(getLeaves(node.children[i], x + i * childW, y, childW, h));
      }
    } else {
      let childH = h / node.children.length;
      for (let i = 0; i < node.children.length; i++) {
        leaves = leaves.concat(getLeaves(node.children[i], x, y + i * childH, w, childH));
      }
    }
    return leaves;
  }

  interface Tab {
    id: string;
    title: string;
    root: SplitNode;
  }
  
  let tabs: Tab[] = [];
  let unlistenTerminated: UnlistenFn | null = null;
  let unlistenStats: UnlistenFn | null = null;

  // Stats state
  interface SshStats { tx: number; rx: number; }
  let sessionStats: Record<string, SshStats> = {};

  // Context menu state
  let contextMenu = { visible: false, x: 0, y: 0, tabId: "" };
  let renameInput = { visible: false, tabId: "", value: "" };

  onMount(async () => {
    unlistenTerminated = await listen("ssh-terminated", (event) => {
      const terminatedId = event.payload as string;
      closeTab(terminatedId);
    });
    unlistenStats = await listen("ssh-stats", (event: any) => {
      const payload = event.payload;
      sessionStats[payload.session_id] = { tx: payload.tx_bytes, rx: payload.rx_bytes };
      sessionStats = { ...sessionStats }; // trigger reactivity
    });
    // Close context menu on click anywhere
    document.addEventListener("click", closeContextMenu);
  });

  onDestroy(() => {
    if (unlistenTerminated) unlistenTerminated();
    if (unlistenStats) unlistenStats();
    document.removeEventListener("click", closeContextMenu);
  });

  let profileToEdit: any = null;

  function openConnectionManager(editProfile: any = null) {
    profileToEdit = editProfile;
    showManager = true;
  }

  function openSettings() {
    showSettings = true;
  }

  function handleConnect(event: CustomEvent<any>) {
    const profile = event.detail;
    showManager = false;
    
    const newTabId = `tab_${Date.now()}`;
    const rootNode: SplitNode = {
      id: `p_${Date.now()}_${Math.floor(Math.random() * 1000)}`,
      type: 'terminal',
      profile
    };
    tabs = [...tabs, { id: newTabId, title: profile.name, root: rootNode }];
    activeTabId = newTabId;
    // Refresh sidebar
    if (sidebarRef) sidebarRef.refresh();
  }

  function handleQuickConnect(event: CustomEvent<any>) {
    const profile = event.detail;
    const newTabId = `tab_${Date.now()}`;
    const rootNode: SplitNode = {
      id: `p_${Date.now()}_${Math.floor(Math.random() * 1000)}`,
      type: 'terminal',
      profile
    };
    tabs = [...tabs, { id: newTabId, title: profile.name, root: rootNode }];
    activeTabId = newTabId;
  }

  function closeTab(id: string) {
    tabs = tabs.filter(t => t.id !== id);
    if (activeTabId === id && tabs.length > 0) {
      activeTabId = tabs[tabs.length - 1].id;
    } else if (tabs.length === 0) {
      activeTabId = "";
    }

    const prefix = `${id}_`;
    const newStats = { ...sessionStats };
    for (const key of Object.keys(newStats)) {
      if (key.startsWith(prefix)) delete newStats[key];
    }
    sessionStats = newStats;
  }

  function handleTabContextMenu(e: MouseEvent, tabId: string) {
    e.preventDefault();
    contextMenu = { visible: true, x: e.clientX, y: e.clientY, tabId };
  }

  function closeContextMenu() {
    contextMenu = { ...contextMenu, visible: false };
  }

  function reconnectTab(tabId: string) {
    const tabIndex = tabs.findIndex(t => t.id === tabId);
    if (tabIndex === -1) return;
    
    const newId = `tab_${Date.now()}`;
    const updatedTab = { ...tabs[tabIndex], id: newId };
    
    tabs = [
      ...tabs.slice(0, tabIndex),
      updatedTab,
      ...tabs.slice(tabIndex + 1)
    ];
    
    if (activeTabId === tabId) {
      activeTabId = newId;
    }
    
    // Clear old stats logic based on compound pane id prefix
    const prefix = `${tabId}_`;
    const newStats = { ...sessionStats };
    for (const key of Object.keys(newStats)) {
      if (key.startsWith(prefix)) delete newStats[key];
    }
    sessionStats = newStats;
    
    closeContextMenu();
  }

  function splitNode(node: SplitNode, targetId: string, direction: SplitDirection): SplitNode | null {
    if (node.id === targetId) {
      if (node.type !== 'terminal') return node;
      
      const newTerminalNode: SplitNode = {
        id: `p_${Date.now()}_${Math.floor(Math.random() * 1000)}`,
        type: 'terminal',
        profile: node.profile
      };
      
      return {
        id: `s_${Date.now()}`,
        type: 'split',
        direction,
        children: [
          { ...node },
          newTerminalNode
        ]
      };
    }
    
    if (node.children) {
      const newChildren = node.children.map(child => splitNode(child, targetId, direction)).filter(Boolean) as SplitNode[];
      return { ...node, children: newChildren };
    }
    
    return node;
  }

  function handleSplitDirect(id: string, direction: SplitDirection) {
    if (!activeTabId) return;
    tabs = tabs.map(tab => {
      if (tab.id === activeTabId) {
        return { ...tab, root: splitNode(tab.root, id, direction)! };
      }
      return tab;
    });
  }

  function removeNode(node: SplitNode, targetId: string): SplitNode | null {
    if (node.id === targetId) return null;
    
    if (node.children) {
      const newChildren = node.children
        .map(child => removeNode(child, targetId))
        .filter(Boolean) as SplitNode[];
        
      if (newChildren.length === 0) return null;
      if (newChildren.length === 1) return newChildren[0]; // flatten
      
      return { ...node, children: newChildren };
    }
    
    return node;
  }

  function handleClosePaneDirect(id: string) {
    if (!activeTabId) return;
    
    const compoundId = `${activeTabId}_${id}`;
    const newStats = { ...sessionStats };
    delete newStats[compoundId];
    sessionStats = newStats;

    tabs = tabs.map(tab => {
      if (tab.id === activeTabId) {
        const newRoot = removeNode(tab.root, id);
        if (!newRoot) {
           setTimeout(() => closeTab(tab.id), 0);
           return tab;
        }
        return { ...tab, root: newRoot };
      }
      return tab;
    });
  }

  function startRename(tabId: string) {
    const tab = tabs.find(t => t.id === tabId);
    if (tab) {
      renameInput = { visible: true, tabId, value: tab.title };
    }
    closeContextMenu();
  }

  function finishRename() {
    if (renameInput.value.trim() !== "") {
      tabs = tabs.map(t => t.id === renameInput.tabId ? { ...t, title: renameInput.value.trim() } : t);
    }
    renameInput = { visible: false, tabId: "", value: "" };
  }

  function handleRenameKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") finishRename();
    if (e.key === "Escape") renameInput = { visible: false, tabId: "", value: "" };
  }

  function formatBytes(bytes: number) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  $: activeTab = tabs.find(t => t.id === activeTabId);
</script>

<main class="app-container">
  <!-- Title/Tab Bar -->
  <header class="tab-bar" data-tauri-drag-region>
    <div class="tabs">
      {#each tabs as tab}
        <div 
          class="tab {activeTabId === tab.id ? 'active' : ''}" 
          on:click={() => activeTabId = tab.id}
          on:contextmenu={(e) => handleTabContextMenu(e, tab.id)}
          aria-hidden="true"
        >
          <div class="tab-content">
            <TerminalIcon size={14} class="icon" />
            {#if renameInput.visible && renameInput.tabId === tab.id}
              <input 
                class="rename-input" 
                type="text" 
                bind:value={renameInput.value} 
                on:blur={finishRename}
                on:keydown={handleRenameKeydown}
                use:focus
              />
            {:else}
              <span class="tab-title">{tab.title}</span>
            {/if}
          </div>
          <button class="close-btn" on:click|stopPropagation={() => closeTab(tab.id)}>
            <X size={14} />
          </button>
        </div>
      {/each}
    </div>
    <div class="actions">
      <button class="icon-btn" on:click={() => openConnectionManager(null)} title="New Connection">
        <Plus size={18} />
      </button>
      <button class="icon-btn {showSidebar ? 'active-action' : ''}" title="Toggle Sidebar" on:click={() => showSidebar = !showSidebar}>
        <PanelRight size={18} />
      </button>
      <button class="icon-btn" title="Settings" on:click={openSettings}>
        <Settings size={18} />
      </button>
    </div>
  </header>

  <!-- Main Content Area -->
  <div class="main-area">
    <div class="content-area">
      {#if tabs.length === 0 && !showManager}
        <div class="empty-state">
          <TerminalIcon size={48} class="empty-icon" />
          <h2>SSH Terminal</h2>
          <button class="primary-btn mt-4" on:click={() => openConnectionManager(null)}>
            Connect to Server
          </button>
        </div>
      {/if}

      {#each tabs as tab (tab.id)}
        <div class="terminal-wrapper" style="display: {activeTabId === tab.id ? 'block' : 'none'}; position: relative; width: 100%; height: 100%; overflow: hidden;">
             {#each getLeaves(tab.root, 0, 0, 100, 100) as leaf (leaf.id)}
                <div class="absolute-pane" style="left: {leaf.x}%; top: {leaf.y}%; width: {leaf.w}%; height: {leaf.h}%;">
                   <div class="pane-controls">
                       <button on:click={() => handleSplitDirect(leaf.id, 'row')} title="Split Right">
                         <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2" ry="2"/><path d="M12 3v18"/></svg>
                       </button>
                       <button on:click={() => handleSplitDirect(leaf.id, 'col')} title="Split Down">
                         <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2" ry="2"/><path d="M3 12h18"/></svg>
                       </button>
                       <button on:click={() => handleClosePaneDirect(leaf.id)} class="close-p" title="Close Pane">
                         <X size={14} />
                       </button>
                   </div>
                   <TerminalArea sessionId="{tab.id}_{leaf.id}" profile={leaf.profile} />
                </div>
             {/each}
        </div>
      {/each}
    </div>

    {#if showSidebar}
      <QuickConnect bind:this={sidebarRef} on:connect={handleQuickConnect} on:editProfile={(e) => openConnectionManager(e.detail)} />
    {/if}
  </div>

  <!-- Context Menu -->
  {#if contextMenu.visible}
    <div class="context-menu" style="left: {contextMenu.x}px; top: {contextMenu.y}px">
      <button class="context-item" on:click={() => startRename(contextMenu.tabId)}>Rename</button>
      <button class="context-item" on:click={() => reconnectTab(contextMenu.tabId)}>Reconnect</button>
      <button class="context-item danger" on:click={() => { closeTab(contextMenu.tabId); closeContextMenu(); }}>Close</button>
    </div>
  {/if}

  {#if showManager}
    <ConnectionManager 
      profileToEdit={profileToEdit}
      on:connect={handleConnect} 
      on:close={() => { showManager = false; profileToEdit = null; if(sidebarRef) sidebarRef.refresh(); }} 
    />
  {/if}

  {#if showSettings}
    <SettingsManager on:close={() => showSettings = false} />
  {/if}

  <!-- Status Bar -->
  <footer class="status-bar">
    {#if activeTab}
      <div class="status-item state-connected">
        <div class="status-dot"></div> Connected
      </div>
      <div class="status-divider"></div>
      <div class="status-item">
        {activeTab.root.profile ? `${activeTab.root.profile.user}@${activeTab.root.profile.host}:${activeTab.root.profile.port}` : "Split View"}
      </div>
      
      {#if sessionStats && Object.keys(sessionStats).some(k => k.startsWith(`${activeTabId}_`))}
        <div class="status-divider"></div>
        <div class="status-item" title="Transmitted">
          TX: <span class="stats-val">{formatBytes(Object.entries(sessionStats).filter(([k]) => k.startsWith(`${activeTabId}_`)).reduce((acc, [_, stats]) => acc + stats.tx, 0))}</span>
        </div>
        <div class="status-item" title="Received">
          RX: <span class="stats-val">{formatBytes(Object.entries(sessionStats).filter(([k]) => k.startsWith(`${activeTabId}_`)).reduce((acc, [_, stats]) => acc + stats.rx, 0))}</span>
        </div>
      {/if}
    {:else}
      <div class="status-item text-muted">No active connection</div>
    {/if}
  </footer>
</main>

<style>
  :global(:root) {
    --bg-darker: #0d0e12;
    --bg-dark: #15171e;
    --bg-surface: #1e2029;
    --bg-hover: #2a2d39;
    --text-main: #e2e8f0;
    --text-muted: #94a3b8;
    --accent: #3b82f6;
    --accent-hover: #2563eb;
    --border: #2a2e3d;
    --danger: #ef4444;
  }

  :global(body) {
    margin: 0;
    padding: 0;
    height: 100vh;
    overflow: hidden;
    background-color: var(--bg-darker);
    color: var(--text-main);
    font-family: 'Inter', system-ui, -apple-system, sans-serif;
  }

  .app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
  }

  .tab-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: var(--bg-darker);
    padding: 0 8px;
    height: 48px;
    border-bottom: 1px solid var(--border);
    user-select: none;
  }

  .tabs {
    display: flex;
    gap: 4px;
    height: 100%;
    align-items: flex-end;
    overflow-x: auto;
    overflow-y: hidden;
  }

  .tabs::-webkit-scrollbar {
    display: none;
  }

  .tab {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 36px;
    min-width: 150px;
    max-width: 200px;
    padding: 0 12px;
    background-color: var(--bg-dark);
    border-radius: 8px 8px 0 0;
    cursor: pointer;
    border: 1px solid transparent;
    border-bottom: none;
    transition: all 0.2s;
    color: var(--text-muted);
  }

  .tab:hover {
    background-color: var(--bg-surface);
  }

  .tab.active {
    background-color: var(--bg-surface);
    color: var(--text-main);
    border-color: var(--border);
    position: relative;
  }

  .tab.active::after {
    content: '';
    position: absolute;
    top: -1px;
    left: 0;
    right: 0;
    height: 2px;
    background-color: var(--accent);
    border-radius: 2px 2px 0 0;
  }

  .tab-content {
    display: flex;
    align-items: center;
    gap: 8px;
    overflow: hidden;
  }

  .tab-title {
    font-size: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .rename-input {
    background: var(--bg-dark);
    border: 1px solid var(--accent);
    border-radius: 3px;
    color: var(--text-main);
    font-size: 12px;
    padding: 2px 6px;
    width: 100px;
    outline: none;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    margin-left: 8px;
  }

  .close-btn:hover {
    background-color: var(--bg-hover);
    color: var(--text-main);
  }

  .actions {
    display: flex;
    gap: 4px;
    padding: 0 8px;
  }

  .icon-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 8px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s;
  }

  .icon-btn:hover {
    background-color: var(--bg-surface);
    color: var(--text-main);
  }

  .active-action {
    color: var(--accent);
  }

  .main-area {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .content-area {
    flex: 1;
    position: relative;
    background-color: var(--bg-surface);
    overflow: hidden;
  }
  
  .terminal-wrapper {
    height: 100%;
    width: 100%;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
  }

  .primary-btn {
    background-color: var(--accent);
    color: white;
    border: none;
    padding: 10px 20px;
    border-radius: 6px;
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .primary-btn:hover {
    background-color: var(--accent-hover);
  }

  /* Context Menu */
  .context-menu {
    position: fixed;
    background-color: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
    z-index: 2000;
    min-width: 140px;
    padding: 4px;
    overflow: hidden;
  }

  .context-item {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    color: var(--text-main);
    padding: 8px 12px;
    font-size: 13px;
    cursor: pointer;
    border-radius: 4px;
  }

  .context-item:hover {
    background-color: var(--bg-hover);
  }

  .context-item.danger {
    color: var(--danger);
  }
  .context-item.danger:hover {
    background-color: rgba(239, 68, 68, 0.1);
  }

  .mt-4 {
    margin-top: 16px;
  }

  /* Status Bar */
  .status-bar {
    height: 24px;
    background-color: var(--bg-dark);
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    padding: 0 12px;
    font-size: 11px;
    color: var(--text-muted);
    user-select: none;
    flex-shrink: 0;
  }

  .status-item {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .status-divider {
    width: 1px;
    height: 12px;
    background-color: var(--border);
    margin: 0 10px;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: #10b981; /* Green */
  }

  .state-connected {
    color: #10b981;
    font-weight: 500;
  }

  .stats-val {
    color: var(--text-main);
    font-variant-numeric: tabular-nums;
  }

  .text-muted {
    color: var(--text-muted);
  }
  /* Pane CSS */
  .absolute-pane {
    position: absolute;
    border: 1px solid var(--border);
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg-darker);
    z-index: 10;
  }
  .absolute-pane .pane-controls {
    position: absolute;
    top: 5px;
    right: 15px;
    z-index: 50;
    display: flex;
    gap: 4px;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.2s;
    background: var(--bg-surface);
    padding: 3px 6px;
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.4);
    border: 1px solid var(--border);
  }
  .absolute-pane:hover .pane-controls {
    opacity: 1;
    pointer-events: auto;
  }
  .absolute-pane .pane-controls button {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 14px;
    padding: 2px 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    transition: background-color 0.1s, color 0.1s;
  }
  .absolute-pane .pane-controls button:hover {
    color: var(--text-main);
    background-color: var(--bg-hover);
  }
  .absolute-pane .pane-controls .close-p:hover {
    color: #ef4444;
    background-color: rgba(239, 68, 68, 0.1);
  }
</style>
