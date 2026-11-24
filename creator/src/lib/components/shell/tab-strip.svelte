<script lang="ts">
	import { untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { IconButton, Menu, MenuItem, Pill, PillTab } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { iconOf } from '$lib/shell/address.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { WholeStrip } from '$lib/shell/strip.svelte.ts';

	/**
	 * The open items' tabs: a strip that scrolls (wheel, trackpad, ‹ ›) and keeps its order – a click never moves a tab, the
	 * front one scrolls into view, a new one opens right of the front one; it stops with a whole tab at its left edge (no
	 * name cut at its start) and its right end fades while more lies beyond. The list names every tab (Close others, Close all); ＋ opens the Library. Ctrl Tab / Ctrl Shift Tab go along the strip,
	 * Ctrl W closes the front tab – where the browser keeps those keys, Alt PageDown / Alt PageUp / Alt W do it.
	 */
	const strip = new WholeStrip('[data-tab-pill]');
	let listOpen = $state(false);
	const tabs = $derived(shell.state.tabs);
	const frontAt = () => strip.indexOf((t) => t.dataset.address === shell.state.front);

	// the front tab scrolls into view, a whole tab at the left edge (never a name cut at its start); the strip's order stays
	$effect(() => {
		void shell.state.front;
		// a tab renamed (its name known after it opened) changes the widths
		void tabs.map((t) => t.label).join('\n');
		untrack(() => (shell.state.front ? strip.show(frontAt()) : strip.measure()));
	});
	$effect(() => {
		void strip.el;
		void tabs.length;
		return untrack(() => strip.watch(() => shell.state.front && strip.show(frontAt(), false)));
	});

	function closeTab(address: string) {
		const next = shell.close(address);
		if (next) void goto(next);
	}
	function closeOthers() {
		shell.closeOthers();
		listOpen = false;
	}
	function closeAll() {
		shell.closeAll();
		listOpen = false;
		void goto('/');
	}
	function step(by: number) {
		const href = shell.step(by);
		if (href) void goto(href);
	}
	function onkeydown(e: KeyboardEvent) {
		const ctrl = (e.ctrlKey || e.metaKey) && !e.altKey;
		const alt = e.altKey && !e.ctrlKey && !e.metaKey;
		if ((ctrl && e.key === 'Tab') || (alt && (e.key === 'PageDown' || e.key === 'PageUp'))) {
			if (!tabs.length) return;
			e.preventDefault();
			step(e.key === 'PageUp' || (e.key === 'Tab' && e.shiftKey) ? -1 : 1);
		} else if (((ctrl && !e.shiftKey) || alt) && e.code === 'KeyW' && shell.state.front) {
			e.preventDefault();
			closeTab(shell.state.front);
		}
	}
</script>

<svelte:window {onkeydown} onresize={() => shell.state.front && strip.show(frontAt(), false)} />

<Pill class="open-items" data-testid="tabs">
	{#if strip.before || strip.after}
		<IconButton label={m['shell.tabs_left']()} icon="chevron-left" disabled={!strip.before} onclick={() => strip.page(-1)} data-testid="tabs-left" />
	{/if}
	<div class="strip" class:after={strip.after} bind:this={strip.el} onscroll={strip.measure} onwheel={strip.wheel}>
		{#each tabs as tab (tab.address)}
			{@const front = tab.address === shell.state.front}
			<PillTab label={tab.label} title={tab.address} icon={iconOf(tab.address)} href={tab.href} on={front} closeLabel={m['shell.close_tab']({ name: tab.label })} onclose={() => closeTab(tab.address)} data-tab-pill data-testid="tab" data-address={tab.address} />
		{/each}
	</div>
	{#if strip.before || strip.after}
		<IconButton label={m['shell.tabs_right']()} icon="chevron-right" disabled={!strip.after} onclick={() => strip.page(1)} data-testid="tabs-right" />
	{/if}
	{#if tabs.length > 1}
		<Menu label={m['shell.tab_list']()} icon="chevron-down" align="start" bind:open={listOpen} testid="tab-list" menuTestid="tab-list-menu">
			{#snippet children(close)}
				{#each tabs as tab (tab.address)}
					<MenuItem icon={iconOf(tab.address)} label={tab.label} href={tab.href} current={tab.address === shell.state.front} onclick={close} />
				{/each}
				<hr />
				<MenuItem label={m['shell.close_others']()} disabled={!shell.state.front} onclick={closeOthers} />
				<MenuItem label={m['shell.close_all']()} onclick={closeAll} data-testid="close-all" />
			{/snippet}
		</Menu>
	{/if}
	<IconButton label={m['shell.open_library']()} icon="plus" onclick={() => shell.open('library')} data-testid="tab-add" />
</Pill>

<style>
	/* the open items: at most a little under half the bar; the strip between the arrows scrolls */
	:global(.kit-pill.open-items) {
		flex: 0 1 auto;
		max-width: 46%;
	}
	.strip {
		position: relative;
		display: flex;
		flex: 0 1 auto;
		gap: var(--kit-seam);
		min-width: 0;
		overflow-x: auto;
		overflow-y: hidden;
		scrollbar-width: none;
		border-radius: var(--kit-r);
		--fade: 2rem;
	}
	.strip::-webkit-scrollbar {
		display: none;
	}
	/* the right end fades while more lies beyond it; the left edge always starts on a whole tab (‹ says more lies before) */
	.strip.after {
		mask-image: linear-gradient(to left, transparent, var(--tool-text) var(--fade));
	}
</style>
