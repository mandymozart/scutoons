<script lang="ts">
	import { goto } from '$app/navigation';
	import { CloseIcon, Icon, clickOutside } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { iconOf } from '$lib/shell/address.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';

	/**
	 * The open items' tabs (the deck's bar.rs tabs): kind icon, name, close (the front tab's path is the bar's middle); the tabs that do not fit wait in the list (every tab, then Close others, Close all); ＋ opens the
	 * Library. Closing the front tab goes to its neighbour, the last one to Home.
	 */
	let strip = $state<HTMLElement>();
	let hidden = $state(0);
	let listOpen = $state(false);
	const tabs = $derived(shell.state.tabs);

	// how many tabs the strip cuts off (the front one never: it comes first when they don't all fit)
	$effect(() => {
		void tabs.length;
		void shell.state.front;
		const el = strip;
		if (!el) return;
		const count = () => {
			const edge = el.getBoundingClientRect().right;
			hidden = [...el.querySelectorAll<HTMLElement>('[data-tab-pill]')].filter((t) => t.getBoundingClientRect().right > edge + 1).length;
		};
		count();
		const observer = new ResizeObserver(count);
		observer.observe(el);
		return () => observer.disconnect();
	});
	const ordered = $derived(hidden ? [...tabs].sort((a, b) => Number(b.address === shell.state.front) - Number(a.address === shell.state.front)) : tabs);

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
</script>

<div class="open-items" data-testid="tabs">
	<div class="strip" bind:this={strip}>
		{#each ordered as tab (tab.address)}
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
	{#if tabs.length}
		<div class="list" {@attach clickOutside(() => (listOpen = false))}>
			<button type="button" class="round" aria-expanded={listOpen} aria-label={m['shell.tab_list']()} title={m['shell.tab_list']()} onclick={() => (listOpen = !listOpen)} data-testid="tab-list">
				<Icon name="chevron-down" size="1rem" />{#if hidden}<span class="count">+{hidden}</span>{/if}
			</button>
			{#if listOpen}
				<div class="menu" role="menu">
					{#each tabs as tab (tab.address)}
						<a role="menuitem" href={tab.href} class:on={tab.address === shell.state.front} onclick={() => (listOpen = false)}><Icon name={iconOf(tab.address)} size="1rem" />{tab.label}</a>
					{/each}
					<hr />
					<button type="button" role="menuitem" disabled={tabs.length < 2 || !shell.state.front} onclick={closeOthers}>{m['shell.close_others']()}</button>
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
		padding: 0.2rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: 1.5rem;
		background: var(--editor-panel-ground);
		text-shadow: var(--editor-halo);
	}
	.strip {
		display: flex;
		flex: 0 1 auto;
		gap: 0.15rem;
		min-width: 0;
		overflow: hidden;
	}
	.item {
		display: flex;
		flex: none;
		align-items: center;
		max-width: 16rem;
		height: var(--editor-row);
		border-radius: 1.2rem;
		color: var(--editor-text);
	}
	.item.front {
		flex-shrink: 1;
		max-width: 40rem;
		background: var(--editor-pressed);
		color: var(--editor-pressed-text);
	}
	.item:not(.front):hover {
		background: var(--editor-hover);
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
	.close,
	.round {
		display: inline-flex;
		flex: none;
		align-items: center;
		justify-content: center;
		gap: 0.1rem;
		min-width: var(--editor-row);
		padding: 0;
		border: 0;
		border-radius: 50%;
		background: transparent;
		color: inherit;
	}
	.round {
		color: var(--editor-text);
	}
	.close:hover,
	.round:hover,
	.round[aria-expanded='true'] {
		background: var(--editor-hover);
	}
	.item.front .close:hover {
		background: transparent;
		opacity: 0.7;
	}
	.count {
		font-size: var(--editor-small);
	}
	.list {
		position: relative;
		flex: none;
	}
	.menu {
		position: absolute;
		z-index: 60;
		top: calc(100% + 0.45rem);
		right: 0;
		display: flex;
		flex-direction: column;
		min-width: 17rem;
		padding: 0.3rem;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: var(--editor-radius);
		background: var(--editor-surface);
	}
	.menu a,
	.menu button {
		display: flex;
		align-items: center;
		justify-content: flex-start;
		gap: 0.5rem;
		padding: 0 0.6rem;
		border: 0;
		border-radius: var(--editor-radius);
		background: transparent;
		color: var(--editor-text);
		text-decoration: none;
		min-height: var(--control-height);
	}
	.menu a:hover,
	.menu button:hover:not(:disabled) {
		background: var(--editor-hover);
	}
	.menu a.on {
		background: var(--editor-selected);
	}
	.menu hr {
		width: 100%;
		margin: 0.25rem 0;
		border: 0;
		border-top: var(--editor-edge) solid var(--editor-line);
	}
</style>
