<script lang="ts">
	import { goto } from '$app/navigation';
	import { IconButton, Menu, MenuItem, Pill, PillTab } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { iconOf } from '$lib/shell/address.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';

	/**
	 * The open items' tabs: a strip that scrolls (wheel, trackpad, ‹ ›) and keeps its order – a click never moves a tab, the
	 * front one scrolls into view, a new one opens right of the front one; the ends fade while more lies beyond them. The
	 * list names every tab (Close others, Close all); ＋ opens the Library. Ctrl Tab / Ctrl Shift Tab go along the strip,
	 * Ctrl W closes the front tab – where the browser keeps those keys, Alt PageDown / Alt PageUp / Alt W do it.
	 */
	let strip = $state<HTMLElement>();
	let listOpen = $state(false);
	let before = $state(false);
	let after = $state(false);
	const tabs = $derived(shell.state.tabs);

	function measure() {
		const el = strip;
		if (!el) return;
		before = el.scrollLeft > 1;
		after = el.scrollLeft + el.clientWidth < el.scrollWidth - 1;
	}
	$effect(() => {
		void tabs.length;
		const el = strip;
		if (!el) return;
		measure();
		const observer = new ResizeObserver(measure);
		observer.observe(el);
		return () => observer.disconnect();
	});
	// the front tab scrolls into view (only as far as needed); the strip's order stays
	$effect(() => {
		const front = shell.state.front;
		void tabs.length;
		const el = strip;
		if (!el || !front) return;
		const tab = [...el.querySelectorAll<HTMLElement>('[data-tab-pill]')].find((t) => t.dataset.address === front);
		if (!tab) return;
		const left = tab.offsetLeft;
		const right = left + tab.offsetWidth;
		const pad = 1.5 * parseFloat(getComputedStyle(document.documentElement).fontSize);
		if (left < el.scrollLeft) el.scrollTo({ left: Math.max(0, left - pad), behavior: 'smooth' });
		else if (right > el.scrollLeft + el.clientWidth) el.scrollTo({ left: right - el.clientWidth + pad, behavior: 'smooth' });
	});

	function page(dir: -1 | 1) {
		const el = strip;
		if (el) el.scrollBy({ left: dir * el.clientWidth * 0.7, behavior: 'smooth' });
	}
	/** a mouse wheel turns into a sideways scroll (a trackpad's own sideways move passes) */
	function onwheel(e: WheelEvent) {
		const el = strip;
		if (!el || Math.abs(e.deltaX) >= Math.abs(e.deltaY) || el.scrollWidth <= el.clientWidth) return;
		e.preventDefault();
		el.scrollLeft += e.deltaY;
	}

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

<svelte:window {onkeydown} />

<Pill class="open-items" data-testid="tabs">
	{#if before || after}
		<IconButton label={m['shell.tabs_left']()} icon="chevron-left" disabled={!before} onclick={() => page(-1)} data-testid="tabs-left" />
	{/if}
	<div class="strip" class:before class:after bind:this={strip} onscroll={measure} {onwheel}>
		{#each tabs as tab (tab.address)}
			{@const front = tab.address === shell.state.front}
			<PillTab label={tab.label} title={tab.address} icon={iconOf(tab.address)} href={tab.href} on={front} closeLabel={m['shell.close_tab']({ name: tab.label })} onclose={() => closeTab(tab.address)} data-tab-pill data-testid="tab" data-address={tab.address} />
		{/each}
	</div>
	{#if before || after}
		<IconButton label={m['shell.tabs_right']()} icon="chevron-right" disabled={!after} onclick={() => page(1)} data-testid="tabs-right" />
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
	/* the ends fade while more lies beyond them */
	.strip.before {
		mask-image: linear-gradient(to right, transparent, var(--tool-text) var(--fade));
	}
	.strip.after {
		mask-image: linear-gradient(to left, transparent, var(--tool-text) var(--fade));
	}
	.strip.before.after {
		mask-image: linear-gradient(to right, transparent, var(--tool-text) var(--fade), var(--tool-text) calc(100% - var(--fade)), transparent);
	}
</style>
