<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { library } from '$lib/editor/library.svelte.ts';
	import { AREAS } from '$lib/shell/address.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import CharactersEditor from '$lib/components/characters/characters-editor.svelte';

	// a character opens from the Library; without one (the old list's address) Home shows the Library's characters
	const open = $derived(page.url.searchParams.has('id') || page.url.searchParams.get('tab') === 'wanted');
	$effect(() => {
		if (open) return;
		library.showOpenings('character');
		// the old list's address leaves no tab behind
		void goto('/', { replaceState: true }).then(() => shell.close(AREAS.characters));
	});
</script>

{#if open}<CharactersEditor />{/if}
