<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import type { AssetFamily } from '@scutoons/media';
	import { library } from '$lib/editor/library.svelte.ts';
	import { copyPath, deleteFamilies, downloadFamilies, renameFamily } from '$lib/editor/library-actions.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A tile's menu (right-click, ⋯, the menu key) with every action: on one file Use, Details, Rename, Replace,
	 * Download, Copy path, Credits, Delete; on several Download, Credits and Delete for all of them.
	 */
	let { x, y, onclose, onreplace }: { x: number; y: number; onclose: () => void; onreplace: (family: AssetFamily) => void } = $props();

	const families = $derived(library.chosenFamilies);
	const one = $derived(families.length === 1 ? families[0] : null);
	let el = $state<HTMLElement>();
	/** rem: kept 0.5rem inside the window */
	let at = $state({ left: 0, top: 0 });
	$effect(() => {
		const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
		const box = el?.getBoundingClientRect();
		const w = box?.width ?? 0;
		const h = box?.height ?? 0;
		at = { left: Math.min(x, window.innerWidth - w - rem / 2) / rem, top: Math.min(y, window.innerHeight - h - rem / 2) / rem };
		el?.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
	});

	function run(action: () => unknown) {
		onclose();
		action();
	}

	function keys(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			event.stopPropagation();
			onclose();
		}
	}
</script>

<svelte:window onpointerdown={(e) => !el?.contains(e.target as Node) && onclose()} onblur={onclose} />

<div bind:this={el} class="menu" role="menu" tabindex="-1" style:left="{at.left}rem" style:top="{at.top}rem" onkeydown={keys} data-testid="library-menu">
	{#if one}
		{#if library.pick}<button type="button" role="menuitem" onclick={() => run(() => library.use(one))}><Icon name="check" size="0.9rem" />{m['library.use_here']()}<kbd>↵</kbd></button>{/if}
		<button type="button" role="menuitem" onclick={() => run(() => (library.details = true))}><Icon name="info" size="0.9rem" />{m['library.details']()}</button>
		<button type="button" role="menuitem" onclick={() => run(() => renameFamily(one))}><Icon name="edit-2" size="0.9rem" />{m['library.rename']()}<kbd>F2</kbd></button>
		<button type="button" role="menuitem" onclick={() => run(() => onreplace(one))}><Icon name="repeat" size="0.9rem" />{m['library.replace']()}</button>
		<button type="button" role="menuitem" onclick={() => run(() => copyPath(one))}><Icon name="copy" size="0.9rem" />{m['library.copy_path']()}</button>
	{/if}
	<button type="button" role="menuitem" onclick={() => run(() => downloadFamilies(families))}><Icon name="download" size="0.9rem" />{families.length > 1 ? m['library.download_n']({ count: families.length }) : m['library.download']()}</button>
	<hr />
	<button type="button" role="menuitem" class="danger" onclick={() => run(() => deleteFamilies(families))}><Icon name="trash-2" size="0.9rem" />{families.length > 1 ? m['library.delete_n']({ count: families.length }) : m['library.delete']()}<kbd>Del</kbd></button>
</div>

<style>
	.menu {
		position: fixed;
		z-index: 80;
		display: flex;
		flex-direction: column;
		min-width: 13rem;
		padding: 0.25rem;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: var(--editor-radius);
		background: var(--tool-popover);
		text-shadow: var(--tool-halo);
		box-shadow: none;
	}
	button {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-height: var(--control-height);
		padding: 0 0.5rem;
		border: 0;
		border-radius: var(--editor-radius);
		background: none;
		color: var(--tool-text);
		font: inherit;
		text-align: start;
		cursor: pointer;
	}
	button:hover:not(:disabled),
	button:focus-visible {
		background: var(--tool-hover);
		outline: none;
	}
	button:disabled {
		color: var(--tool-muted);
		cursor: default;
	}
	.danger {
		color: var(--tool-danger);
	}
	kbd {
		margin-inline-start: auto;
		color: var(--tool-muted);
		font-family: var(--editor-mono, ui-monospace, monospace);
		font-size: var(--editor-label);
	}
	hr {
		width: 100%;
		margin: 0.2rem 0;
		border: 0;
		border-top: var(--editor-edge) solid var(--tool-line);
	}
</style>
