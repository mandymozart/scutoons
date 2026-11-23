<script lang="ts">
	import { goto } from '$app/navigation';
	import { CloseIcon, Icon, clickOutside } from '@scutoons/ui';
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

<div class="open-items" class:none={!tabs.length} data-testid="tabs">
	{#if before || after}
		<button type="button" class="round step" disabled={!before} aria-label={m['shell.tabs_left']()} title={m['shell.tabs_left']()} onclick={() => page(-1)} data-testid="tabs-left"><Icon name="chevron-left" size="0.9rem" /></button>
	{/if}
	<div class="strip" class:before class:after bind:this={strip} onscroll={measure} {onwheel}>
		{#each tabs as tab (tab.address)}
			{@const front = tab.address === shell.state.front}
			<div class="item" class:front data-tab-pill title={tab.address} data-testid="tab" data-address={tab.address}>
				<a class="open" href={tab.href} aria-current={front ? 'page' : undefined}>
					<Icon name={iconOf(tab.address)} size="1rem" />
					<span class="name">{tab.label}</span>
				</a>
				<button type="button" class="close" aria-label={m['shell.close_tab']({ name: tab.label })} title={m['shell.close_tab']({ name: tab.label })} onclick={() => closeTab(tab.address)}><CloseIcon size="0.8rem" /></button>
			</div>
		{/each}
	</div>
	{#if before || after}
		<button type="button" class="round step" disabled={!after} aria-label={m['shell.tabs_right']()} title={m['shell.tabs_right']()} onclick={() => page(1)} data-testid="tabs-right"><Icon name="chevron-right" size="0.9rem" /></button>
	{/if}
	{#if tabs.length > 1}
		<div class="list" {@attach clickOutside(() => (listOpen = false))}>
			<button type="button" class="round" aria-expanded={listOpen} aria-label={m['shell.tab_list']()} title={m['shell.tab_list']()} onclick={() => (listOpen = !listOpen)} data-testid="tab-list"><Icon name="chevron-down" size="1rem" /></button>
			{#if listOpen}
				<div class="menu" role="menu">
					{#each tabs as tab (tab.address)}
						<a role="menuitem" href={tab.href} class:on={tab.address === shell.state.front} onclick={() => (listOpen = false)}><Icon name={iconOf(tab.address)} size="1rem" /><span class="name">{tab.label}</span></a>
					{/each}
					<hr />
					<button type="button" role="menuitem" disabled={!shell.state.front} onclick={closeOthers}>{m['shell.close_others']()}</button>
					<button type="button" role="menuitem" onclick={closeAll} data-testid="close-all">{m['shell.close_all']()}</button>
				</div>
			{/if}
		</div>
	{/if}
	<button type="button" class="round" aria-label={m['shell.open_library']()} title={m['shell.open_library']()} onclick={() => shell.open('library')} data-testid="tab-add"><Icon name="plus" size="1rem" /></button>
</div>

<style>
	.open-items {
		display: flex;
		flex: 0 1 auto;
		align-items: center;
		gap: 0.15rem;
		min-width: 0;
		max-width: 46%;
		padding: 0.2rem;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: 1.5rem;
		background: var(--tool-panel);
		text-shadow: var(--tool-halo);
	}
	/* the halo on its icons too (the deck's ink ring covers text and icons) */
	.open-items :global(svg) {
		filter: var(--tool-icon-halo);
	}
	.strip {
		position: relative;
		display: flex;
		flex: 0 1 auto;
		gap: 0.15rem;
		min-width: 0;
		overflow-x: auto;
		overflow-y: hidden;
		scrollbar-width: none;
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
	.item {
		display: flex;
		flex: none;
		align-items: center;
		max-width: 14rem;
		height: var(--editor-row);
		border-radius: var(--editor-radius);
		color: var(--tool-muted);
	}
	/* the front item: a thin outline, the others muted */
	.item.front {
		color: var(--tool-text);
		box-shadow: inset 0 0 0 var(--editor-edge) var(--tool-line);
	}
	.item:not(.front):hover {
		background: var(--tool-hover);
	}
	.open {
		display: flex;
		flex: 1 1 auto;
		align-items: center;
		gap: 0.4rem;
		min-width: 0;
		padding: 0 0.2rem 0 0.75rem;
		color: inherit;
		text-decoration: none;
	}
	.name {
		flex: 0 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	/* a tab's close shows on the front tab and under the pointer */
	.item:not(.front) .close {
		opacity: 0;
	}
	.item:hover .close,
	.close:focus-visible {
		opacity: 1;
	}
	.close,
	.round {
		display: inline-flex;
		flex: none;
		align-items: center;
		justify-content: center;
		min-width: var(--editor-row);
		min-height: var(--editor-row);
		padding: 0;
		border: 0;
		border-radius: 50%;
		background: transparent;
		color: inherit;
	}
	.round {
		color: var(--tool-text);
	}
	.round.step,
	.close {
		min-width: 1.4rem;
	}
	.round:disabled {
		opacity: 0.35;
	}
	.close:hover,
	.round:hover:not(:disabled),
	.round[aria-expanded='true'] {
		background: var(--tool-hover);
	}
	.list {
		position: relative;
		flex: none;
	}
	.menu {
		position: absolute;
		z-index: 60;
		top: calc(100% + 0.45rem);
		left: 0;
		display: flex;
		flex-direction: column;
		width: 17rem;
		max-height: calc(100dvh - var(--shell-top) - var(--status-room));
		overflow: auto;
		padding: 0.3rem;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: var(--editor-radius);
		background: var(--tool-popover);
	}
	.menu a,
	.menu button {
		display: flex;
		flex: none;
		align-items: center;
		justify-content: flex-start;
		gap: 0.5rem;
		min-height: var(--control-height);
		padding: 0 0.6rem;
		border: 0;
		border-radius: var(--editor-radius);
		background: transparent;
		color: var(--tool-text);
		text-decoration: none;
	}
	.menu a:hover,
	.menu button:hover:not(:disabled) {
		background: var(--tool-hover);
	}
	.menu a.on {
		box-shadow: inset 0 0 0 var(--editor-edge) var(--tool-line);
	}
	.menu hr {
		width: 100%;
		margin: 0.25rem 0;
		border: 0;
		border-top: var(--editor-edge) solid var(--tool-line);
	}
</style>
