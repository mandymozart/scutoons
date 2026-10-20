<script lang="ts">
	// The Workspace control (STUDIO-IN-CREATOR §2.2 place 1): the member's local folder for every attempt – choose it once
	// (Chrome / Edge: the browser's folder picker), its name shown, change it; after a browser restart one click allows it
	// again. Elsewhere (Firefox, Safari, phones) one line says it needs Chrome or Edge. With a folder: this computer's
	// label (the records of files saved here name it – `machine`, #121; this browser's localStorage, "this browser").
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { toast } from '$lib/toasts.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';

	let { sb }: { sb: StoryboardPage } = $props();

	const ws = $derived(sb.workspaceState);
	const name = $derived(sb.workspaceName);
	async function run(job: () => Promise<void>) {
		try {
			await job();
		} catch (e) {
			toast(e instanceof Error ? e.message : String(e), { type: 'error' });
		}
	}
</script>

{#if ws === 'unsupported'}
	<span class="note" title={m['storyboard.workspace_hint']()} data-testid="workspace-unsupported"><Icon name="folder" /> {m['storyboard.workspace_unsupported']()}</span>
{:else if ws === 'ask'}
	<button type="button" class="secondary" title={m['storyboard.workspace_allow_hint']()} onclick={() => run(() => sb.allowWorkspace())} data-testid="workspace-allow"><Icon name="folder" /> {m['storyboard.workspace_allow']({ name: name ?? '' })}</button>
{:else}
	<button
		type="button"
		class="secondary"
		title={`${m['storyboard.workspace']()}: ${m['storyboard.workspace_hint']()}${ws === 'ready' ? ` – ${m['storyboard.workspace_change']()}` : ''}`}
		onclick={() => run(() => sb.chooseWorkspace())}
		data-testid="workspace-choose"
	>
		<Icon name="folder" /> <span class="name">{ws === 'ready' && name ? name : m['storyboard.workspace_choose']()}</span>
	</button>
	{#if ws === 'ready'}
		<input class="machine" type="text" maxlength="64" aria-label={m['storyboard.machine']()} title={m['storyboard.machine_hint']()} value={sb.machine} onchange={(e) => { sb.setMachine(e.currentTarget.value); e.currentTarget.value = sb.machine; }} data-testid="workspace-machine" />
	{/if}
{/if}

<style>
	button {
		max-width: 16rem;
	}
	.machine {
		width: 9rem;
	}
	.name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.note {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
</style>
