<script lang="ts">
  import TerminalArea from './TerminalArea.svelte';
  import { createEventDispatcher } from 'svelte';
  
  export let node: any;
  export let activeTabId: string;
  
  const dispatch = createEventDispatcher();

  function split(nodeId: string, direction: 'row' | 'col') {
    dispatch('split', { id: nodeId, direction });
  }

  function closePane(nodeId: string) {
    dispatch('closePane', { id: nodeId });
  }

</script>

{#if node.type === 'split'}
  <div class="split-container {node.direction}">
    {#each node.children as child (child.id)}
      <div class="split-child">
        <svelte:self 
          node={child} 
          activeTabId={activeTabId}
          on:split 
          on:closePane 
        />
      </div>
    {/each}
  </div>
{:else if node.type === 'terminal'}
  <div class="pane-wrapper">
    <!-- Controls overlay for splitting/closing -->
    <div class="pane-controls">
      <button on:click={() => split(node.id, 'row')} title="Split Right">◫</button>
      <button on:click={() => split(node.id, 'col')} title="Split Down">⊟</button>
      <button on:click={() => closePane(node.id)} class="close-p" title="Close Pane">×</button>
    </div>
    
    <!-- We prefix the sessionId with activeTabId to reuse existing ssh-stats tracking per node.id -->
    <TerminalArea sessionId="{activeTabId}_{node.id}" profile={node.profile} />
  </div>
{/if}

<style>
  .split-container {
    display: flex;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .split-container.row {
    flex-direction: row;
  }

  .split-container.col {
    flex-direction: column;
  }

  .split-child {
    flex: 1 1 0;
    min-width: 0;
    min-height: 0;
    border: 1px solid var(--border); /* Optional subtle divider */
    display: flex;
    flex-direction: column;
  }

  .pane-wrapper {
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .pane-controls {
    position: absolute;
    top: 5px;
    right: 15px;
    z-index: 10;
    display: flex;
    gap: 4px;
    opacity: 0;
    transition: opacity 0.2s;
    background: var(--bg-surface);
    padding: 2px 4px;
    border-radius: 4px;
    box-shadow: 0 2px 5px rgba(0,0,0,0.3);
  }

  .pane-wrapper:hover .pane-controls {
    opacity: 1;
  }

  .pane-controls button {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 14px;
    padding: 2px 4px;
  }

  .pane-controls button:hover {
    color: var(--text-main);
  }

  .pane-controls .close-p:hover {
    color: #ef4444;
  }
</style>
