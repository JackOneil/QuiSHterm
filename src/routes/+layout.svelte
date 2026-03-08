<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  
  function handleContextMenu(e: MouseEvent) {
    // Povolíme pravé tlačítko pouze u elementů s třídou xterm-screen nebo elementů v nich zabalených
    const target = e.target as HTMLElement;
    if (target && target.closest('.xterm-screen')) {
      return; // Necháme nativní prohlížečové menu
    }
    // Jinak zakážeme kontextové menu všude jinde
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
