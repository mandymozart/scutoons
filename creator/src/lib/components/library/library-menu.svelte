<script lang="ts">
	import { MenuItem, MenuPanel } from '@scutoons/ui/editor';
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

<div bind:this={el} class="menu" style:left="{at.left}rem" style:top="{at.top}rem">
	<MenuPanel width="13rem" tabindex={-1} onkeydown={keys} data-testid="library-menu">
		{#if one}
			<MenuItem icon="info" label={m['library.details']()} onclick={() => run(() => (library.details = true))} />
			<MenuItem icon="edit-2" label={m['library.rename']()} kbd="F2" onclick={() => run(() => renameFamily(one))} />
			<MenuItem icon="repeat" label={m['library.replace']()} onclick={() => run(() => onreplace(one))} />
			<MenuItem icon="copy" label={m['library.copy_path']()} onclick={() => run(() => copyPath(one))} />
		{/if}
		<MenuItem icon="download" label={families.length > 1 ? m['library.download_n']({ count: families.length }) : m['library.download']()} onclick={() => run(() => downloadFamilies(families))} />
		<hr />
		<MenuItem class="danger" icon="trash-2" label={families.length > 1 ? m['library.delete_n']({ count: families.length }) : m['library.delete']()} kbd="Del" onclick={() => run(() => deleteFamilies(families))} />
	</MenuPanel>
</div>

<style>
	.menu {
		position: fixed;
		z-index: 80;
	}
	.menu :global(.danger) {
		color: var(--tool-danger);
	}
</style>