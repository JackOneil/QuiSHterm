<script lang="ts">
  import { onMount, createEventDispatcher } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { Eye, EyeOff, Plus, Server, Trash2, X } from "lucide-svelte";

  const dispatch = createEventDispatcher();

  export let profileToEdit: any = null;

  const terminalOptions = [
    "xterm-256color",
    "xterm",
    "xterm-color",
    "screen",
    "screen-256color",
    "vt100",
    "vt220",
    "ansi"
  ];

  let profiles: any[] = [];
  let editingProfile: any = null;
  let selectedProfileId = "";
  let revealPassword = false;

  function createEmptyProfile() {
    return {
      id: `prof_${Date.now()}`,
      name: "New Connection",
      host: "127.0.0.1",
      port: 22,
      user: "root",
      password: "",
      private_key: "",
      auth_type: null,
      terminal_type: "xterm-256color"
    };
  }

  function normalizeProfile(profile: any) {
    return {
      ...createEmptyProfile(),
      ...profile,
      password: profile?.password || "",
      private_key: profile?.private_key || "",
      auth_type: profile?.auth_type || null,
      terminal_type: profile?.terminal_type || "xterm-256color"
    };
  }

  async function loadProfiles() {
    try {
      profiles = await invoke("load_profiles");
    } catch (e) {
      console.error(e);
      profiles = [];
    }
  }

  async function browseKey() {
    const selected = await open({
      multiple: false,
      directory: false,
    });

    if (selected && typeof selected === "string") {
      editingProfile.private_key = selected;
    }
  }

  function close() {
    dispatch("close");
  }

  function selectProfile(profile: any) {
    editingProfile = normalizeProfile(profile);
    selectedProfileId = editingProfile.id;
    revealPassword = false;
  }

  function startNewProfile() {
    editingProfile = createEmptyProfile();
    selectedProfileId = editingProfile.id;
    revealPassword = false;
  }

  async function saveProfile() {
    try {
      profiles = await invoke("save_profile", { profile: editingProfile });
      const savedProfile = profiles.find((profile) => profile.id === editingProfile.id);
      selectProfile(savedProfile || editingProfile);
    } catch (e) {
      console.error(e);
    }
  }

  async function deleteProfile(id: string) {
    try {
      profiles = await invoke("delete_profile", { id });
      if (selectedProfileId === id) {
        if (profiles.length > 0) {
          selectProfile(profiles[0]);
        } else {
          startNewProfile();
        }
      }
    } catch (e) {
      console.error(e);
    }
  }

  function connectTo(profile: any) {
    dispatch("connect", normalizeProfile(profile));
  }

  function getProfilePreview(profile: any) {
    if (editingProfile && editingProfile.id === profile.id) {
      return normalizeProfile(editingProfile);
    }

    return normalizeProfile(profile);
  }

  function revertProfile() {
    const existing = profiles.find((profile) => profile.id === selectedProfileId);
    if (existing) {
      selectProfile(existing);
    } else {
      startNewProfile();
    }
  }

  $: hasUnsavedProfile = !!editingProfile && !profiles.some((profile) => profile.id === editingProfile.id);

  onMount(async () => {
    await loadProfiles();

    if (profileToEdit) {
      selectProfile(profileToEdit);
      return;
    }

    if (profiles.length > 0) {
      selectProfile(profiles[0]);
    } else {
      startNewProfile();
    }
  });
</script>

<div class="modal-backdrop" on:click={close} aria-hidden="true">
  <div class="modal" on:click|stopPropagation on:keydown|stopPropagation role="dialog" aria-label="Connections" tabindex="-1">
    <div class="modal-header">
      <div>
        <h2>Connections</h2>
        <p class="modal-subtitle">Compact connection profiles with agent/Pageant, key, and password fallback.</p>
      </div>
      <button class="icon-btn" type="button" aria-label="Close connections" on:click={close}>
        <X size={18} />
      </button>
    </div>

    <div class="modal-content">
      <aside class="profile-list-pane">
        <div class="pane-toolbar">
          <button class="outline-btn" type="button" on:click={startNewProfile}>
            <Plus size={16} class="mr-2" /> New Connection
          </button>
        </div>

        <div class="profiles-list">
          {#each profiles as profile}
            {@const profilePreview = getProfilePreview(profile)}
            <div class="profile-card {selectedProfileId === profile.id ? 'selected' : ''}">
              <button class="profile-main" type="button" on:click={() => selectProfile(profile)}>
                <span class="profile-name"><Server size={14} class="mr-2" /> {profilePreview.name}</span>
                <span class="profile-host">{profilePreview.host}:{profilePreview.port}</span>
              </button>
              <div class="profile-actions">
                <button class="primary-btn sm" type="button" on:click={() => connectTo(profilePreview)}>Connect</button>
                <button class="icon-btn danger" type="button" aria-label={`Delete ${profile.name}`} on:click={() => deleteProfile(profile.id)}>
                  <Trash2 size={15} />
                </button>
              </div>
            </div>
          {/each}

          {#if profiles.length === 0}
            <div class="empty-list-state">No saved connections yet.</div>
          {/if}
        </div>
      </aside>

      <section class="editor-pane">
        {#if editingProfile}
          <div class="editor-header">
            <div>
              <h3>{hasUnsavedProfile ? 'Create Connection' : 'Edit Connection'}</h3>
              <p class="editor-hint">Authentication order: agent/Pageant, configured key, default `~/.ssh` keys, then password.</p>
            </div>
            <div class="editor-actions">
              <button class="secondary-btn" type="button" on:click={revertProfile}>Revert</button>
              {#if !hasUnsavedProfile}
                <button class="secondary-btn" type="button" on:click={() => connectTo(editingProfile)}>Connect</button>
              {/if}
              <button class="primary-btn" type="button" on:click={saveProfile}>Save</button>
            </div>
          </div>

          <div class="editor-body">
            <section class="editor-section">
              <div class="section-title-row">
                <h4>Basic</h4>
              </div>

              <label class="field wide">
                <span>Name</span>
                <input type="text" bind:value={editingProfile.name} />
              </label>

              <div class="field-grid field-grid-main">
                <label class="field field-host">
                  <span>Host / IP</span>
                  <input type="text" bind:value={editingProfile.host} />
                </label>
                <label class="field field-port">
                  <span>Port</span>
                  <input type="number" bind:value={editingProfile.port} min="1" max="65535" />
                </label>
                <label class="field field-user">
                  <span>User</span>
                  <input type="text" bind:value={editingProfile.user} />
                </label>
              </div>
            </section>

            <section class="editor-section">
              <div class="section-title-row">
                <h4>Authentication</h4>
                <span class="section-note">Password and key can stay empty if agent/Pageant or `~/.ssh` should be used first.</span>
              </div>

              <label class="field">
                <span>Password or key passphrase</span>
                <div class="input-with-action">
                  <input type={revealPassword ? "text" : "password"} bind:value={editingProfile.password} placeholder="Optional. Prompted at connect time if needed." />
                  <button class="icon-btn ghost" type="button" aria-label={revealPassword ? 'Hide password' : 'Reveal password'} on:click={() => revealPassword = !revealPassword}>
                    {#if revealPassword}
                      <EyeOff size={16} />
                    {:else}
                      <Eye size={16} />
                    {/if}
                  </button>
                </div>
              </label>

              <label class="field">
                <span>Private key file</span>
                <div class="input-with-button">
                  <input type="text" bind:value={editingProfile.private_key} placeholder="Optional. Leave empty to rely on agent/Pageant or ~/.ssh." spellcheck="false" />
                  <button class="secondary-btn" type="button" on:click={browseKey}>Browse</button>
                </div>
              </label>
            </section>

            <section class="editor-section">
              <div class="section-title-row">
                <h4>Terminal</h4>
              </div>

              <label class="field compact">
                <span>Default PTY terminal type</span>
                <select bind:value={editingProfile.terminal_type}>
                  {#each terminalOptions as terminalOption}
                    <option value={terminalOption}>{terminalOption}</option>
                  {/each}
                </select>
              </label>
            </section>
          </div>
        {/if}
      </section>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background-color: rgba(0, 0, 0, 0.72);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    width: min(1080px, 94vw);
    min-height: 620px;
    max-height: 88vh;
    background-color: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 16px;
    box-shadow: 0 28px 80px rgba(0, 0, 0, 0.55);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 18px 22px;
    border-bottom: 1px solid var(--border);
    background: linear-gradient(180deg, rgba(59, 130, 246, 0.08), transparent 85%);
  }

  .modal-header h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
  }

  .modal-subtitle {
    margin: 6px 0 0;
    color: var(--text-muted);
    font-size: 13px;
  }

  .modal-content {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 320px minmax(0, 1fr);
  }

  .profile-list-pane {
    border-right: 1px solid var(--border);
    background: rgba(13, 14, 18, 0.45);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .pane-toolbar {
    padding: 18px;
    border-bottom: 1px solid var(--border);
  }

  .profiles-list {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
  }

  .profile-card {
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--bg-dark);
    padding: 8px 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .profile-card.selected {
    border-color: rgba(59, 130, 246, 0.65);
    box-shadow: inset 0 0 0 1px rgba(59, 130, 246, 0.28);
  }

  .profile-main {
    border: 0;
    background: transparent;
    color: inherit;
    display: flex;
    flex-direction: column;
    gap: 2px;
    align-items: flex-start;
    text-align: left;
    cursor: pointer;
    padding: 0;
    min-width: 0;
    flex: 1 1 auto;
  }

  .profile-name {
    display: flex;
    align-items: center;
    font-weight: 600;
    color: var(--text-main);
    gap: 6px;
    min-width: 0;
    font-size: 13px;
  }

  .profile-host {
    font-size: 12px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .profile-actions {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
    align-items: center;
    flex-shrink: 0;
  }

  .editor-pane {
    min-width: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .editor-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 18px;
    padding: 20px 22px 16px;
    border-bottom: 1px solid var(--border);
  }

  .editor-header h3 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }

  .editor-hint {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  .editor-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .editor-body {
    padding: 20px 22px 24px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .editor-section {
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 16px;
    background: rgba(13, 14, 18, 0.35);
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .section-title-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }

  .section-title-row h4 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }

  .section-note {
    font-size: 12px;
    color: var(--text-muted);
  }

  .field-grid {
    display: grid;
    gap: 12px;
  }

  .field-grid-main {
    grid-template-columns: minmax(0, 1.6fr) 110px minmax(0, 1fr);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field span {
    font-size: 12px;
    color: var(--text-muted);
  }

  .wide {
    width: 100%;
  }

  input,
  select {
    width: 100%;
    min-width: 0;
    box-sizing: border-box;
    background-color: var(--bg-dark);
    border: 1px solid var(--border);
    color: var(--text-main);
    padding: 9px 12px;
    border-radius: 8px;
    font-size: 14px;
    transition: border-color 0.2s, box-shadow 0.2s;
  }

  input:focus,
  select:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.14);
  }

  .input-with-action,
  .input-with-button {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
    align-items: center;
  }

  .empty-list-state {
    padding: 18px 14px;
    border: 1px dashed var(--border);
    border-radius: 12px;
    color: var(--text-muted);
    text-align: center;
    font-size: 13px;
  }

  .icon-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 7px;
    border-radius: 8px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .icon-btn:hover {
    background: var(--bg-hover);
    color: var(--text-main);
  }

  .icon-btn.ghost {
    border: 1px solid var(--border);
    background: var(--bg-dark);
  }

  .icon-btn.danger:hover {
    color: #f87171;
  }

  .primary-btn,
  .secondary-btn,
  .outline-btn {
    border-radius: 8px;
    padding: 9px 14px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }

  .primary-btn {
    border: none;
    background: var(--accent);
    color: white;
  }

  .primary-btn:hover {
    background: var(--accent-hover);
  }

  .primary-btn.sm {
    padding: 7px 11px;
  }

  .secondary-btn {
    background: transparent;
    color: var(--text-main);
    border: 1px solid var(--border);
  }

  .secondary-btn:hover,
  .outline-btn:hover {
    background: var(--bg-hover);
  }

  .outline-btn {
    width: 100%;
    border: 1px dashed var(--border);
    background: transparent;
    color: var(--text-main);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  @media (max-width: 920px) {
    .modal-content {
      grid-template-columns: 1fr;
    }

    .profile-list-pane {
      border-right: 0;
      border-bottom: 1px solid var(--border);
      max-height: 260px;
    }

    .field-grid-main {
      grid-template-columns: 1fr;
    }
  }

  :global(.mr-2) { margin-right: 8px; }
</style>