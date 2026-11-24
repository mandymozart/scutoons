<script lang="ts">
	// /edit/<kind>:<id>?w=<workspace>&sel=<path> – the creator's editor (research/EDITOR.md §4, #282). The key is the subject,
	// `w` the workspace (Sound design for rigs and modules), `sel` the selection. Creators and admins only (the creator app is
	// gated). A key that is no known kind reads in the writing page's column (lap 2 C14).
	import { page } from '$app/state';
	import { parseEditKey, workspaceOf } from '$lib/editor/edit-key.ts';
	import { m } from '$lib/paraglide/messages.js';
	import SoundWorkspace from '$lib/components/sound/sound-workspace.svelte';
	import WritingPage from '$lib/components/writing/writing-page.svelte';

	const key = $derived(page.params.key ?? '');
	const target = $derived(parseEditKey(key));
	const workspace = $derived(target ? workspaceOf(target, page.url.searchParams.get('w')) : null);
</script>

{#if target && workspace === 'sound'}
	{#key target.key}
		<SoundWorkspace {target} />
	{/key}
{:else}
	<WritingPage title={m['creator.sound_cant_edit']()} sub={key} testid="edit-unknown">
		<p class="muted">{m['creator.sound_cant_edit_hint']({ key })}</p>
	</WritingPage>
{/if}

<style>
	.muted {
		margin: 0;
		color: var(--tool-muted);
	}
</style>
