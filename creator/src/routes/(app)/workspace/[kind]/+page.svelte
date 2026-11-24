<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Button } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { emptyWorkspace, findSeed } from '$lib/shell/workspaces.ts';
	import { workspaceName } from '$lib/shell/workspace-names.ts';

	// a workspace that needs an item first, nothing of it open: one quiet line and the next step – Search, on its items
	const kind = $derived(emptyWorkspace(page.url.pathname));
	// a workspace with a start of its own, or none: Home
	$effect(() => {
		if (!kind) void goto('/', { replaceState: true });
	});
</script>

{#if kind}
	<div class="ws-empty" data-testid="workspace-empty">
		<p class="sc-empty">{m['shell.ws_empty']({ name: workspaceName(kind) })}</p>
		<Button icon="search" onclick={() => shell.toggleLauncher(true, findSeed(kind))} data-testid="workspace-empty-find">{m['shell.ws_find']()}</Button>
	</div>
{/if}

<style>
	.ws-empty {
		position: fixed;
		z-index: 30;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		pointer-events: none;
	}
	.ws-empty > :global(*) {
		pointer-events: auto;
	}
	.ws-empty .sc-empty {
		margin: 0;
	}
</style>
