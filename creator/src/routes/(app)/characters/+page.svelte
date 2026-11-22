<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { library } from '$lib/editor/library.svelte.ts';
	import CharactersEditor from '$lib/components/characters/characters-editor.svelte';

	// a character opens from the Library; without one (the old list's address) Home shows the Library's characters
	const open = $derived(page.url.searchParams.has('id') || page.url.searchParams.get('tab') === 'wanted');
	$effect(() => {
		if (open) return;
		library.showOpenings('character');
		void goto('/', { replaceState: true });
	});
</script>

{#if open}<CharactersEditor />{/if}
