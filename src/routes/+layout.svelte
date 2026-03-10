<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  
  function handleContextMenu(e: MouseEvent) {
    e.preventDefault();
  }

  onMount(() => {
    document.addEventListener('contextmenu', handleContextMenu);
  });

  onDestroy(() => {
    document.removeEventListener('contextmenu', handleContextMenu);
  });
</script>

<slot />
