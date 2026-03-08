<script lang="ts">
  import { onMount, createEventDispatcher } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { Server, User, Plus, Trash2, X } from "lucide-svelte";

  const dispatch = createEventDispatcher();
  let profiles: any[] = [];
  let isEditing = false;
  let editingProfile: any = null;

  export let profileToEdit: any = null;

  async function browseKey() {
    const selected = await open({
      multiple: false,
      directory: false,
    });
    if (selected && typeof selected === 'string') {
      editingProfile.private_key = selected;
    }
  }

  onMount(async () => {
    try {
      profiles = await invoke("load_profiles");
    } catch(e) {
      console.error(e);
    }
    
    if (profileToEdit) {
      startEdit(profileToEdit);
    }
  });

  function close() {
    dispatch("close");
  }

  function startEdit(profile = null) {
    if (profile) {
      editingProfile = { ...profile };
    } else {
      editingProfile = {
        id: `prof_${Date.now()}`,
        name: "New Connection",
        host: "127.0.0.1",
        port: 22,
        user: "root",
        password: "",
        private_key: ""
      };
    }
    isEditing = true;
  }

  async function saveProfile() {
    try {
      profiles = await invoke("save_profile", { profile: editingProfile });
      isEditing = false;
    } catch(e) {
      console.error(e);
    }
  }

  async function deleteProfile(id: string) {
    try {
      profiles = await invoke("delete_profile", { id });
      if (isEditing && editingProfile.id === id) {
        isEditing = false;
      }
    } catch(e) {
      console.error(e);
    }
  }

  function connectTo(profile: any) {
    dispatch("connect", profile);
  }
</script>

<div class="modal-backdrop" on:click={close} aria-hidden="true">
  <div class="modal" on:click|stopPropagation on:keydown|stopPropagation role="dialog" aria-label="Connections" tabindex="-1">
    <div class="modal-header">
      <h2>Connections</h2>
      <button class="icon-btn" on:click={close}><X size={18} /></button>
    </div>

    <div class="modal-content">
      {#if isEditing}
        <div class="edit-form">
          <label>
            Name
            <input type="text" bind:value={editingProfile.name} />
          </label>
          <div class="row">
            <label class="flex-3">
              Host / IP
              <input type="text" bind:value={editingProfile.host} />
            </label>
            <label class="flex-1">
              Port
              <input type="number" bind:value={editingProfile.port} />
            </label>
          </div>
          <div class="row">
            <label class="flex-1">
              User
              <input type="text" bind:value={editingProfile.user} />
            </label>
            <label class="flex-1">
              Password (or passphrase)
              <input type="password" bind:value={editingProfile.password} />
            </label>
          </div>
          <label>
            Private Key (Path to file, e.g. ~/.ssh/id_rsa)
            <div style="display: flex; gap: 8px;">
              <input type="text" bind:value={editingProfile.private_key} placeholder="Leave empty to use password" style="flex: 1;" />
              <button type="button" class="secondary-btn" on:click={browseKey} style="padding: 0 16px;">Browse...</button>
            </div>
          </label>
          <div class="modal-actions">
            <button class="secondary-btn" on:click={() => { isEditing = false; if(profileToEdit) close(); }}>Cancel</button>
            <button class="primary-btn" on:click={saveProfile}>Save</button>
          </div>
        </div>
      {:else}
        <div class="profiles-list">
          {#each profiles as p}
            <div class="profile-card">
              <div class="profile-info" on:click={() => startEdit(p)} aria-hidden="true">
                <div class="profile-name"><Server size={14} class="mr-2"/> {p.name}</div>
                <div class="profile-host"><User size={12} class="mr-1"/> {p.user}@{p.host}:{p.port}</div>
              </div>
              <div class="card-actions">
                <button class="primary-btn sm" on:click={() => connectTo(p)}>Connect</button>
                <button class="icon-btn danger" on:click={() => deleteProfile(p.id)}><Trash2 size={16} /></button>
              </div>
            </div>
          {/each}
        </div>
        
        <div class="add-new">
          <button class="outline-btn" on:click={() => startEdit(null)}>
            <Plus size={16} class="mr-2"/> Add Connection
          </button>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background-color: var(--bg-surface);
    width: 600px;
    max-width: 90vw;
    border-radius: 12px;
    border: 1px solid var(--border);
    box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  .modal-header h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 500;
  }

  .modal-content {
    padding: 20px;
  }

  .icon-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 6px;
    border-radius: 6px;
    display: flex;
    align-items: center;
  }
  
  .icon-btn:hover {
    background-color: var(--bg-hover);
    color: var(--text-main);
  }

  .icon-btn.danger:hover {
    color: var(--danger);
  }

  .profiles-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 20px;
    max-height: 400px;
    overflow-y: auto;
  }

  .profile-card {
    background-color: var(--bg-dark);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px 16px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .profile-info {
    cursor: pointer;
    flex: 1;
  }

  .profile-name {
    font-weight: 500;
    margin-bottom: 4px;
    display: flex;
    align-items: center;
  }

  .profile-host {
    font-size: 12px;
    color: var(--text-muted);
    display: flex;
    align-items: center;
  }

  .card-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .add-new {
    display: flex;
    justify-content: center;
  }

  .edit-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .row {
    display: flex;
    gap: 12px;
  }

  .flex-1 { flex: 1; }
  .flex-3 { flex: 3; }

  label {
    display: flex;
    flex-direction: column;
    font-size: 13px;
    color: var(--text-muted);
    gap: 6px;
  }

  input {
    background-color: var(--bg-dark);
    border: 1px solid var(--border);
    color: var(--text-main);
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 14px;
    transition: border-color 0.2s;
  }

  input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 10px;
  }

  .primary-btn {
    background-color: var(--accent);
    color: white;
    border: none;
    padding: 8px 16px;
    border-radius: 6px;
    font-weight: 500;
    cursor: pointer;
  }

  .primary-btn.sm {
    padding: 6px 12px;
    font-size: 13px;
  }

  .primary-btn:hover {
    background-color: var(--accent-hover);
  }
  
  .secondary-btn {
    background-color: transparent;
    color: var(--text-main);
    border: 1px solid var(--border);
    padding: 8px 16px;
    border-radius: 6px;
    cursor: pointer;
  }

  .secondary-btn:hover {
    background-color: var(--bg-hover);
  }

  .outline-btn {
    background-color: transparent;
    border: 1px dashed var(--border);
    color: var(--text-muted);
    width: 100%;
    padding: 12px;
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    font-weight: 500;
    transition: all 0.2s;
  }

  .outline-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
    background-color: rgba(59, 130, 246, 0.05);
  }

  :global(.mr-1) { margin-right: 4px; }
  :global(.mr-2) { margin-right: 8px; }
</style>
