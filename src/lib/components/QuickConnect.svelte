<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { ChevronDown, ChevronRight, FolderPlus, X, Server, ArrowRight } from "lucide-svelte";

  const dispatch = createEventDispatcher();

  let profiles: any[] = [];
  let folders: any[] = [];
  let showNewFolder = false;
  let newFolderName = "";
  let newFolderColor = "#3b82f6";

  // Context Menu state
  let ctxMenu = { visible: false, x: 0, y: 0, type: '', id: '', item: null as any };


  // Move-to-folder state (replaces broken drag-and-drop)
  let movingProfileId: string | null = null;
  let renameFolderState = { active: false, id: '', name: '', color: '' };

  const folderColors = [
    "#3b82f6", "#ef4444", "#10b981", "#f59e0b", "#8b5cf6", "#06b6d4", "#ec4899", "#f97316"
  ];

  onMount(async () => {
    await refresh();
    // Restore saved sidebar width
    const savedWidth = localStorage.getItem('sidebar_width');
    if (savedWidth) {
      sidebarWidth = parseInt(savedWidth, 10);
    }
  });

  onDestroy(() => {
    // Clean up drag listeners if needed
    document.removeEventListener('mousemove', onDrag);
    document.removeEventListener('mouseup', onDragEnd);
    document.removeEventListener('click', closeCtxMenu);
  });

  function closeCtxMenu() {
    ctxMenu = { ...ctxMenu, visible: false };
  }

  function handleContextMenu(e: MouseEvent, type: 'folder' | 'profile', item: any) {
    e.preventDefault();
    e.stopPropagation();
    ctxMenu = {
      visible: true,
      x: e.clientX,
      y: e.clientY,
      type,
      id: item.id,
      item
    };
  }

  // Resize state
  let sidebarWidth = 220;
  let isDragging = false;

  function onDragStart(e: MouseEvent) {
    e.preventDefault();
    isDragging = true;
    document.addEventListener('mousemove', onDrag);
    document.addEventListener('mouseup', onDragEnd);
  }

  function onDrag(e: MouseEvent) {
    if (!isDragging) return;
    // Sidebar is on the right - width = window width - mouse X
    const newWidth = window.innerWidth - e.clientX;
    sidebarWidth = Math.max(140, Math.min(500, newWidth));
  }

  function onDragEnd() {
    isDragging = false;
    document.removeEventListener('mousemove', onDrag);
    document.removeEventListener('mouseup', onDragEnd);
    localStorage.setItem('sidebar_width', String(sidebarWidth));
  }

  export async function refresh() {
    try {
      profiles = await invoke("load_profiles");
    } catch (e) {
      console.error("Failed to load profiles:", e);
    }
    try {
      let settings: any = await invoke("load_settings");
      folders = settings.folders || [];
    } catch (e) {
      console.error("Failed to load folders:", e);
    }
  }

  function connectProfile(profile: any) {
    dispatch("connect", profile);
  }

  async function saveFolders() {
    try {
      let settings: any = await invoke("load_settings");
      settings.folders = folders;
      await invoke("save_settings", { settings });
    } catch (e) {
      console.error("Failed to save folders:", e);
    }
  }

  function addFolder() {
    if (newFolderName.trim() === "") return;
    folders = [...folders, {
      id: `folder_${Date.now()}`,
      name: newFolderName.trim(),
      color: newFolderColor,
      profile_ids: [],
      collapsed: false
    }];
    newFolderName = "";
    showNewFolder = false;
    saveFolders();
  }

  function deleteFolder(folderId: string) {
    folders = folders.filter((f: any) => f.id !== folderId);
    saveFolders();
  }

  function toggleFolder(folderId: string) {
    folders = folders.map((f: any) => f.id === folderId ? { ...f, collapsed: !f.collapsed } : f);
  }

  function getUncategorizedProfiles() {
    const categorizedIds = new Set<string>();
    folders.forEach((f: any) => f.profile_ids?.forEach((id: string) => categorizedIds.add(id)));
    return profiles.filter(p => !categorizedIds.has(p.id));
  }

  function getProfilesForFolder(folder: any) {
    return profiles.filter(p => folder.profile_ids?.includes(p.id));
  }

  function startMove(profileId: string) {
    movingProfileId = movingProfileId === profileId ? null : profileId;
  }

  function moveToFolder(folderId: string) {
    if (!movingProfileId) return;
    // Remove from all folders
    folders = folders.map((f: any) => ({
      ...f,
      profile_ids: (f.profile_ids || []).filter((id: string) => id !== movingProfileId)
    }));
    // Add to target
    if (folderId !== "__uncategorized__") {
      folders = folders.map((f: any) => f.id === folderId ? { ...f, profile_ids: [...(f.profile_ids || []), movingProfileId] } : f);
    }
    movingProfileId = null;
    saveFolders();
  }

  // --- Profile Actions ---
  function editProfile(profile: any) {
    dispatch("editProfile", profile);
    closeCtxMenu();
  }

  async function removeProfile(profileId: string) {
    if (confirm("Are you sure you want to delete this profile?")) {
      try {
        await invoke("delete_profile", { id: profileId });
        await refresh();
      } catch(e) { console.error(e); }
    }
    closeCtxMenu();
  }

  // --- Folder Actions ---
  function editFolder(folder: any) {
    renameFolderState = { active: true, id: folder.id, name: folder.name, color: folder.color };
    closeCtxMenu();
  }

  function saveFolderEdit() {
    folders = folders.map((f: any) => f.id === renameFolderState.id 
      ? { ...f, name: renameFolderState.name, color: renameFolderState.color } 
      : f);
    saveFolders();
    renameFolderState.active = false;
  }
</script>

<div class="sidebar-wrapper" style="width: {sidebarWidth}px" role="region" aria-label="Sidebar">
  <div class="resize-handle" on:mousedown={onDragStart} aria-hidden="true"></div>
  <div class="sidebar-panel">
  <div class="sidebar-header">
    <span class="sidebar-title">Connections</span>
    <button class="icon-btn-sm" on:click={() => showNewFolder = !showNewFolder} title="New Folder">
      <FolderPlus size={14} />
    </button>
  </div>

  {#if showNewFolder}
    <div class="new-folder-form">
      <input type="text" placeholder="Folder name" bind:value={newFolderName} on:keydown={(e) => e.key === 'Enter' && addFolder()} />
      <div class="color-picker">
        {#each folderColors as c}
          <button class="color-dot {newFolderColor === c ? 'active' : ''}" style="background-color: {c}" on:click={() => newFolderColor = c} title={c}></button>
        {/each}
      </div>
      <button class="add-folder-btn" on:click={addFolder}>Create</button>
    </div>
  {/if}

  {#if movingProfileId}
    <div class="move-bar">
      <span class="move-label">Move to:</span>
      {#each folders as folder}
        <button class="move-target" on:click={() => moveToFolder(folder.id)}>
          <span class="folder-dot-sm" style="background-color: {folder.color}"></span>
          {folder.name}
        </button>
      {/each}
      <button class="move-target" on:click={() => moveToFolder('__uncategorized__')}>
        Uncategorized
      </button>
      <button class="move-cancel" on:click={() => movingProfileId = null}>Cancel</button>
    </div>
  {/if}

  <div class="sidebar-list">
    {#each folders as folder}
      <div class="folder-group" role="group" aria-label={folder.name}>
        {#if renameFolderState.active && renameFolderState.id === folder.id}
          <div class="folder-header editing" on:click|stopPropagation on:keydown|stopPropagation role="button" tabindex="0">
            <input type="text" bind:value={renameFolderState.name} class="rename-input" 
                   on:keydown={(e) => e.key === 'Enter' && saveFolderEdit()}>
            <div class="color-picker-mini">
               {#each folderColors as c}
                 <button class="color-dot {renameFolderState.color === c ? 'active' : ''}" 
                         style="background-color: {c}" on:click={() => renameFolderState.color = c} title="{c}"></button>
               {/each}
            </div>
            <button class="add-folder-btn" on:click={saveFolderEdit}>Save</button>
            <button class="icon-btn-sm" on:click={() => renameFolderState.active = false}><X size={12}/></button>
          </div>
        {:else}
          <div class="folder-header" on:click={() => toggleFolder(folder.id)} on:contextmenu={(e) => handleContextMenu(e, 'folder', folder)} aria-hidden="true">
          <div class="folder-label">
            <span class="folder-dot" style="background-color: {folder.color}"></span>
            {#if folder.collapsed}
              <ChevronRight size={12} />
            {:else}
              <ChevronDown size={12} />
            {/if}
            <span class="folder-name">{folder.name}</span>
          </div>
          <button class="icon-btn-sm text-danger" on:click|stopPropagation={() => deleteFolder(folder.id)} title="Delete folder">
            <X size={12} />
          </button>
        </div>
        {/if}
        {#if !folder.collapsed && !renameFolderState.active}
          <div class="folder-items">
            {#each getProfilesForFolder(folder) as profile}
              <div class="profile-item {movingProfileId === profile.id ? 'moving' : ''}" on:contextmenu={(e) => handleContextMenu(e, 'profile', profile)} role="button" tabindex="0" on:keydown|stopPropagation>
                <button class="profile-connect" on:click={() => connectProfile(profile)}>
                  <Server size={12} />
                  <span class="profile-name">{profile.name}</span>
                </button>
                <button class="move-btn" on:click|stopPropagation={() => startMove(profile.id)} title="Move to folder">
                  <ArrowRight size={11} />
                </button>
              </div>
            {/each}
            {#if getProfilesForFolder(folder).length === 0}
              <div class="folder-empty">No connections</div>
            {/if}
          </div>
        {/if}
      </div>
    {/each}

    <!-- Uncategorized -->
    {#if getUncategorizedProfiles().length > 0}
      <div class="section-label">Uncategorized</div>
    {/if}
    {#each getUncategorizedProfiles() as profile}
      <div class="profile-item {movingProfileId === profile.id ? 'moving' : ''}" on:contextmenu={(e) => handleContextMenu(e, 'profile', profile)} role="button" tabindex="0" on:keydown|stopPropagation>
        <button class="profile-connect" on:click={() => connectProfile(profile)}>
          <Server size={12} />
          <span class="profile-name">{profile.name}</span>
        </button>
        <button class="move-btn" on:click|stopPropagation={() => startMove(profile.id)} title="Move to folder">
          <ArrowRight size={11} />
        </button>
      </div>
    {/each}

    {#if profiles.length === 0}
      <div class="sidebar-empty">No saved connections.</div>
    {/if}
  </div>

  <!-- Sidebar Context Menu -->
  {#if ctxMenu.visible}
    <div class="ctx-menu" style="left: {ctxMenu.x}px; top: {ctxMenu.y}px" on:click|stopPropagation on:keydown|stopPropagation role="menu" tabindex="-1">
      {#if ctxMenu.type === 'profile'}
        <button class="ctx-item" on:click={() => { connectProfile(ctxMenu.item); closeCtxMenu(); }}>Connect</button>
        <button class="ctx-item" on:click={() => editProfile(ctxMenu.item)}>Edit</button>
        <button class="ctx-item danger" on:click={() => removeProfile(ctxMenu.id)}>Remove</button>
      {:else if ctxMenu.type === 'folder'}
        <button class="ctx-item" on:click={() => editFolder(ctxMenu.item)}>Edit</button>
        <button class="ctx-item danger" on:click={() => { deleteFolder(ctxMenu.id); closeCtxMenu(); }}>Remove</button>
      {/if}
    </div>
  {/if}

</div>
</div>

<style>
  .sidebar-wrapper {
    display: flex;
    flex-shrink: 0;
    position: relative;
    height: 100%;
  }

  .resize-handle {
    width: 4px;
    cursor: col-resize;
    background-color: transparent;
    transition: background-color 0.15s;
    flex-shrink: 0;
  }
  .resize-handle:hover {
    background-color: var(--accent);
  }

  .sidebar-panel {
    flex: 1;
    min-width: 0;
    background-color: var(--bg-dark);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
  }

  .sidebar-title {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
  }

  .icon-btn-sm {
    background: none; border: none; color: var(--text-muted);
    cursor: pointer; padding: 4px; border-radius: 4px;
    display: flex; align-items: center;
  }
  .icon-btn-sm:hover { background-color: var(--bg-hover); color: var(--text-main); }

  .new-folder-form {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .new-folder-form input {
    padding: 6px 8px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: white;
    font-size: 12px;
  }
  .rename-input {
    width: 60px;
    padding: 2px 4px;
    background: var(--bg-surface);
    border: 1px solid var(--accent);
    color: white;
    font-size: 12px;
    border-radius: 3px;
  }
  .new-folder-form input:focus, .rename-input:focus { outline: none; border-color: var(--accent); }

  .color-picker, .color-picker-mini {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }

  .color-dot {
    width: 16px; height: 16px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    padding: 0;
  }
  .color-dot.active { border-color: white; }

  .add-folder-btn {
    background: var(--accent); color: white; border: none;
    padding: 4px 8px; border-radius: 4px; font-size: 11px;
    cursor: pointer; font-weight: 500;
  }
  .add-folder-btn:hover { background: var(--accent-hover); }

  /* Move bar */
  .move-bar {
    padding: 6px 8px;
    background-color: rgba(59, 130, 246, 0.1);
    border-bottom: 1px solid var(--accent);
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .move-label {
    font-size: 10px;
    text-transform: uppercase;
    color: var(--accent);
    font-weight: 600;
    letter-spacing: 0.5px;
    margin-bottom: 2px;
  }

  .move-target {
    display: flex;
    align-items: center;
    gap: 6px;
    background: none;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 8px;
    color: var(--text-main);
    font-size: 11px;
    cursor: pointer;
  }
  .move-target:hover {
    background-color: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .folder-dot-sm {
    width: 6px; height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .move-cancel {
    background: none;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 4px 8px;
    color: var(--text-muted);
    font-size: 11px;
    cursor: pointer;
    margin-top: 2px;
  }
  .move-cancel:hover { background-color: var(--bg-hover); color: var(--text-main); }

  .sidebar-list {
    flex: 1;
    overflow-y: auto;
    padding: 6px 0;
  }

  .folder-group {
    margin-bottom: 2px;
  }

  .folder-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 12px;
    cursor: pointer;
    user-select: none;
  }
  .folder-header:hover { background-color: var(--bg-hover); }

  .folder-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-main);
  }

  .folder-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .folder-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .folder-items {
    padding-left: 12px;
  }

  .folder-empty {
    font-size: 11px;
    color: var(--text-muted);
    font-style: italic;
    padding: 4px 12px;
  }

  .section-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    padding: 8px 12px 4px;
    font-weight: 600;
  }

  .profile-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 2px 4px 2px 0;
    border-radius: 4px;
    transition: all 0.15s;
  }
  .profile-item:hover { background-color: var(--bg-hover); }
  .profile-item.moving { background-color: rgba(59, 130, 246, 0.15); border: 1px solid var(--accent); }

  .profile-connect {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    text-align: left;
    background: none;
    border: none;
    padding: 5px 8px;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 12px;
  }
  .profile-connect:hover { color: var(--text-main); }

  .profile-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .move-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 3px;
    border-radius: 3px;
    display: flex;
    align-items: center;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .profile-item:hover .move-btn { opacity: 1; }
  .move-btn:hover { background-color: var(--bg-surface); color: var(--accent); }

  .sidebar-empty {
    text-align: center;
    padding: 20px 12px;
    color: var(--text-muted);
    font-size: 12px;
    font-style: italic;
  }

  .text-danger { color: #ef4444; }
  .text-danger:hover { color: #fca5a5; }

  /* Context Menu */
  .ctx-menu {
    position: fixed;
    background-color: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0,0,0,0.5);
    z-index: 10000;
    min-width: 120px;
    padding: 4px;
    display: flex;
    flex-direction: column;
  }
  .ctx-item {
    background: none;
    border: none;
    color: var(--text-main);
    text-align: left;
    padding: 6px 10px;
    font-size: 12px;
    cursor: pointer;
    border-radius: 4px;
  }
  .ctx-item:hover { background-color: var(--bg-hover); }
  .ctx-item.danger { color: #ef4444; }
  .ctx-item.danger:hover { background-color: rgba(239, 68, 68, 0.1); }
  
  .editing { flex-direction: column; align-items: flex-start; gap: 4px; }
  .color-picker-mini { display: flex; gap: 4px; flex-wrap: wrap; margin: 4px 0; }
</style>
