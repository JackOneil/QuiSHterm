<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let visible: boolean = false;
  export let x: number = 0;
  export let y: number = 0;
  export let suggestions: string[] = [];
  export let selectedIndex: number = 0;

  const dispatch = createEventDispatcher();

  function selectSuggestion(index: number) {
    if (index >= 0 && index < suggestions.length) {
      dispatch("select", suggestions[index]);
    }
  }
</script>

{#if visible && suggestions.length > 0}
  <div class="autocomplete-popup" style="left: {x}px; top: {y}px;">
    {#each suggestions as suggestion, index}
      <div 
        class="suggestion-item" 
        class:selected={index === selectedIndex}
        on:mousedown|preventDefault={() => selectSuggestion(index)}
      >
        <div class="icon">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m9 18 6-6-6-6"/></svg>
        </div>
        <span class="text">{suggestion}</span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .autocomplete-popup {
    position: fixed;
    z-index: 10000;
    background-color: #1e1e2e;
    border: 1px solid #313244;
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
    min-width: 150px;
    max-height: 250px;
    overflow-y: hidden;
    display: flex;
    flex-direction: column;
    padding: 4px;
    font-family: 'Fira Code', 'Cascadia Code', monospace;
    font-size: 13px;
    pointer-events: none; /* Let xterm handle mouse events unless specifically interacting early */
  }

  .autocomplete-popup:hover {
    pointer-events: auto;
  }

  .suggestion-item {
    padding: 6px 12px;
    color: #cdd6f4;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 8px;
    border-radius: 4px;
    transition: background-color 0.1s;
  }

  .suggestion-item.selected {
    background-color: #89b4fa;
    color: #11111b;
  }

  .suggestion-item:not(.selected):hover {
    background-color: #313244;
  }
  
  .suggestion-item.selected .icon {
    color: #11111b;
  }

  .icon {
    color: #a6adc8;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
