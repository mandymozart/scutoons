<script lang="ts" module>
	export interface Tab {
		id: string;
		label: string;
		disabled?: boolean;
	}
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * Tabs (WAI-ARIA tablist, arrow keys / Home / End). `children(activeId)` renders the active panel.
	 */
	let {
		tabs,
		value = $bindable(),
		label,
		onchange,
		children
	}: {
		tabs: Tab[];
		value?: string;
		/** accessible name of the tab list */
		label?: string;
		onchange?: (id: string) => void;
		children?: Snippet<[string]>;
	} = $props();

	const uid = $props.id();
	const active = $derived(value ?? tabs.find((t) => !t.disabled)?.id ?? '');
	let buttons: HTMLButtonElement[] = $state([]);

	function select(id: string) {
		value = id;
		onchange?.(id);
	}

	function onKeydown(event: KeyboardEvent, index: number) {
		const enabled = tabs.map((t, i) => (t.disabled ? -1 : i)).filter((i) => i >= 0);
		const at = enabled.indexOf(index);
		let next: number | undefined;
		if (event.key === 'ArrowRight') next = enabled[(at + 1) % enabled.length];
		else if (event.key === 'ArrowLeft') next = enabled[(at - 1 + enabled.length) % enabled.length];
		else if (event.key === 'Home') next = enabled[0];
		else if (event.key === 'End') next = enabled[enabled.length - 1];
		if (next === undefined) return;
		event.preventDefault();
		select(tabs[next].id);
		buttons[next]?.focus();
	}
</script>

<div class="sc-tabs">
	<div class="list" role="tablist" aria-label={label}>
		{#each tabs as tab, index (tab.id)}
			<button
				bind:this={buttons[index]}
				type="button"
				role="tab"
				id="{uid}-tab-{tab.id}"
				aria-selected={active === tab.id}
				aria-controls="{uid}-panel"
				tabindex={active === tab.id ? 0 : -1}
				disabled={tab.disabled}
				class:active={active === tab.id}
				onclick={() => select(tab.id)}
				onkeydown={(event) => onKeydown(event, index)}
			>
				{tab.label}
			</button>
		{/each}
	</div>
	{#if children}
		<div class="panel" role="tabpanel" id="{uid}-panel" aria-labelledby="{uid}-tab-{active}" tabindex="0">
			{@render children(active)}
		</div>
	{/if}
</div>

<style>
	.list {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
		margin-bottom: var(--space-1);
	}
	button {
		box-sizing: border-box;
		min-height: var(--control-height);
		margin: 0;
		padding: var(--space-025) var(--space-1);
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--border-muted);
		border-radius: var(--radius-none);
		font: inherit;
		line-height: 1.2;
		cursor: pointer;
	}
	button:hover:not(:disabled),
	button:focus-visible {
		border-color: var(--ink);
	}
	button.active {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	button:focus-visible,
	.panel:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}
</style>
