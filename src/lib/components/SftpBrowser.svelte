<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, createEventDispatcher } from "svelte";

  export let sessionId: string;
  export let serverName: string;

  const dispatch = createEventDispatcher();

  let currentPath = "/";
  let files: any[] = [];
  let isLoading = false;
  let errorMsg = "";

  onMount(() => {
    loadDirectory("/");
  });

  async function loadDirectory(path: string) {
    if (isLoading) return;
    isLoading = true;
    errorMsg = "";
    try {
      if (!path.endsWith("/")) path += "/";
      const result: any[] = await invoke("sftp_list_dir", { sessionId, path });
      files = result;
      currentPath = path;
    } catch(err: any) {
      errorMsg = err.toString();
    } finally {
      isLoading = false;
    }
  }

  function handleRowClick(file: any) {
    if (file.is_dir) {
      if (file.name === "." || file.name === "..") {
        if (currentPath === "/") return;
        const parts = currentPath.split("/").filter(Boolean);
        parts.pop();
        loadDirectory("/" + parts.join("/"));
      } else {
        loadDirectory(currentPath + file.name + "/");
      }
    }
  }

  async function handleDownload(file: any) {
    if (file.is_dir) {
      errorMsg = "Downloading folders is not supported yet.";
      return;
    }
    
    try {
      isLoading = true;
      const remotePath = currentPath + file.name;
      // Get base64 string
      const b64: string = await invoke("sftp_download_file", { sessionId, remotePath });
      
      // Convert to blob and trigger download in browser
      const byteCharacters = atob(b64);
      const byteNumbers = new Array(byteCharacters.length);
      for (let i = 0; i < byteCharacters.length; i++) {
        byteNumbers[i] = byteCharacters.charCodeAt(i);
      }
      const byteArray = new Uint8Array(byteNumbers);
      const blob = new Blob([byteArray]);
      
      const link = document.createElement('a');
      link.href = URL.createObjectURL(blob);
      link.download = file.name;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      
    } catch(err: any) {
      errorMsg = "Download failed: " + err;
    } finally {
      isLoading = false;
    }
  }

  function formatSize(bytes: number) {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
  }

  function formatDate(unixTime: number) {
    return new Date(unixTime * 1000).toLocaleString();
  }
  
  function triggerUpload() {
    let input = document.createElement("input");
    input.type = "file";
    input.onchange = async (e: any) => {
      const file = e.target.files[0];
      if (!file) return;
      
      try {
        isLoading = true;
        const reader = new FileReader();
        reader.onload = async (ev: any) => {
          const b64 = ev.target.result.split(',')[1]; // Remove data URL prefix
          const remotePath = currentPath + file.name;
          await invoke("sftp_upload_file", { sessionId, remotePath, base64Data: b64 });
          loadDirectory(currentPath); // Refresh
        };
        reader.readAsDataURL(file);
      } catch(err: any) {
         errorMsg = "Upload failed: " + err.toString();
         isLoading = false;
      }
    };
    input.click();
  }
</script>

<div class="sftp-modal-overlay">
  <div class="sftp-modal-content">
    <div class="modal-header">
      <h2>SFTP Browser: {serverName}</h2>
      <button class="close-btn" on:click={() => dispatch('close')}>
         <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
      </button>
    </div>

    <div class="toolbar">
      <div class="path-bar">
        <button on:click={() => loadDirectory("..")} title="Up Directory" disabled={currentPath === "/"}>
           <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 19-7-7 7-7"/><path d="M19 12H5"/></svg>
        </button>
        <input type="text" value={currentPath} on:keydown={(e) => e.key === 'Enter' && loadDirectory(e.currentTarget.value)} />
        <button on:click={() => loadDirectory(currentPath)}>Refresh</button>
      </div>
      <div class="actions">
        <button on:click={triggerUpload} class="btn-primary" disabled={isLoading}>
           <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" x2="12" y1="3" y2="15"/></svg>
           Upload File
        </button>
      </div>
    </div>

    {#if errorMsg}
      <div class="error-box">{errorMsg}</div>
    {/if}

    <div class="file-table-container">
      <table class="file-table">
        <thead>
          <tr>
            <th width="50%">Name</th>
            <th width="20%">Size</th>
            <th width="20%">Modified</th>
            <th width="10%">Actions</th>
          </tr>
        </thead>
        <tbody>
          {#if isLoading}
            <tr><td colspan="4" class="text-center loading">Loading...</td></tr>
          {:else if files.length === 0}
            <tr><td colspan="4" class="text-center">Directory is empty</td></tr>
          {:else}
            <!-- Always show parent dir if not root -->
            {#if currentPath !== "/"}
              <tr class="file-row" on:dblclick={() => loadDirectory("..")}>
                <td>
                  <div class="file-name">
                     <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="icon-dir"><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/></svg>
                     ..
                  </div>
                </td>
                <td>-</td>
                <td>-</td>
                <td></td>
              </tr>
            {/if}

            {#each files as file}
              {#if file.name !== "." && file.name !== ".."}
                <tr class="file-row" on:dblclick={() => handleRowClick(file)}>
                  <td>
                    <div class="file-name">
                      {#if file.is_dir}
                         <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="icon-dir"><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/></svg>
                      {:else}
                         <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="icon-file"><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/></svg>
                      {/if}
                      {file.name}
                    </div>
                  </td>
                  <td>{file.is_dir ? "-" : formatSize(file.size)}</td>
                  <td>{formatDate(file.modified)}</td>
                  <td class="action-cell">
                    {#if !file.is_dir}
                      <button on:click={(e) => { e.stopPropagation(); handleDownload(file); }} title="Download">
                         <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/></svg>
                      </button>
                    {/if}
                  </td>
                </tr>
              {/if}
            {/each}
          {/if}
        </tbody>
      </table>
    </div>
  </div>
</div>

<style>
  .sftp-modal-overlay {
    position: fixed;
    top: 0; left: 0; right: 0; bottom: 0;
    background-color: rgba(0,0,0,0.7);
    z-index: 1000;
    display: flex;
    justify-content: center;
    align-items: center;
    backdrop-filter: blur(2px);
  }

  .sftp-modal-content {
    background-color: var(--bg-darker);
    border: 1px solid var(--border);
    border-radius: 8px;
    width: 800px;
    max-width: 90vw;
    height: 600px;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 10px 25px rgba(0,0,0,0.5);
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
    background-color: var(--bg-surface);
  }

  h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
  }

  .close-btn:hover {
    color: var(--text-main);
  }

  .toolbar {
    display: flex;
    justify-content: space-between;
    padding: 12px 20px;
    background-color: var(--bg-dark);
    border-bottom: 1px solid var(--border);
  }

  .path-bar {
    display: flex;
    gap: 8px;
    flex: 1;
    margin-right: 16px;
  }

  .path-bar input {
    flex: 1;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--text-main);
    padding: 6px 10px;
    border-radius: 4px;
    font-size: 14px;
  }

  .path-bar button, .actions button {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    color: var(--text-main);
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
  }

  .path-bar button:hover:not(:disabled) {
    background: var(--bg-hover);
  }

  .btn-primary {
    background: var(--accent) !important;
    border-color: var(--accent) !important;
    color: white !important;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--accent-hover) !important;
  }

  .error-box {
    background-color: rgba(239, 68, 68, 0.1);
    color: #ef4444;
    padding: 10px 20px;
    border-bottom: 1px solid #ef4444;
    font-size: 13px;
  }

  .file-table-container {
    flex: 1;
    overflow-y: auto;
    padding: 0;
  }

  .file-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
    text-align: left;
  }

  .file-table th {
    position: sticky;
    top: 0;
    background-color: var(--bg-surface);
    padding: 10px 20px;
    border-bottom: 1px solid var(--border);
    color: var(--text-muted);
    font-weight: 500;
  }

  .file-table td {
    padding: 8px 20px;
    border-bottom: 1px solid var(--border);
  }

  .file-row {
    cursor: pointer;
    user-select: none;
  }

  .file-row:hover {
    background-color: var(--bg-surface);
  }

  .file-name {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  :global(.icon-dir) {
    color: #fbbf24;
  }

  :global(.icon-file) {
    color: #94a3b8;
  }

  .text-center {
    text-align: center;
  }

  .loading {
    color: var(--text-muted);
    padding: 40px !important;
  }

  .action-cell button {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
  }

  .action-cell button:hover {
    background: var(--bg-hover);
    color: var(--text-main);
  }
</style>
