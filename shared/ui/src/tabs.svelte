<script lang="ts" module>
	export interface Tab {
		id: string;
		label: string;
		disabled?: boolean;
		/** an Icon name before the label */
		icon?: string;
		/** shown as "(n)" after the label; in the `grid` variant a small trailing number */
		count?: number;
		testid?: string;
		title?: string;
		/** a dashed error frame (e.g. a look without images) */
		warning?: boolean;
	}
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from './icon.svelte';

	/**
	 * Tabs (WAI-ARIA tablist, arrow keys / Home / End). `children(activeId)` renders the active panel; without it the
	 * page renders the panel itself. `tab(tab, active)` replaces a tab's content (icon + label + count by default);
	 * `after` sits in the row behind the tabs, outside the tab list (e.g. "+ add").
	 * variant: `buttons` (default), `ruled` (a 2px rule under the row, the creator's pages), `sub` (a 1px rule, the
	 * level below), `grid` (a framed grid of equal tabs, `--tabs-min` wide: the slice inspector).
	 */
	let {
		tabs,
		value = $bindable(),
		label,
		variant = 'buttons',
		onchange,
		tab,
		after,
		children
	}: {
		tabs: Tab[];
		value?: string;
		/** accessible name of the tab list */
		label?: string;
		variant?: 'buttons' | 'ruled' | 'sub' | 'grid';
		onchange?: (id: string) => void;
		tab?: Snippet<[Tab, boolean]>;
		after?: Snippet;
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

<div class="sc-tabs" data-variant={variant} class:has-panel={!!children}>
	<div class="row">
		<div class="list" role="tablist" aria-label={label}>
			{#each tabs as t, index (t.id)}
				<button
					bind:this={buttons[index]}
					type="button"
					role="tab"
					id="{uid}-tab-{t.id}"
					aria-selected={active === t.id}
					aria-controls={children ? `${uid}-panel` : undefined}
					tabindex={active === t.id ? 0 : -1}
					disabled={t.disabled}
					title={t.title ?? (variant === 'grid' ? t.label : undefined)}
					class:active={active === t.id}
					class:warning={t.warning}
					data-testid={t.testid}
					onclick={() => select(t.id)}
					onkeydown={(event) => onKeydown(event, index)}
				>
					{#if tab}{@render tab(t, active === t.id)}{:else}{#if t.icon}<Icon name={t.icon} />{/if}<span class="label">{t.label}</span>{#if t.count !== undefined}{#if variant === 'grid'}<span class="count">{t.count}</span>{:else}{' '}({t.count}){/if}{/if}{/if}
				</button>
			{/each}
		</div>
		{@render after?.()}
	</div>
	{#if children}
		<div class="panel" role="tabpanel" id="{uid}-panel" aria-labelledby="{uid}-tab-{active}" tabindex="0">
			{@render children(active)}
		</div>
	{/if}
</div>

<style>
	.row {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-025);
		align-items: center;
	}
	.has-panel > .row {
		margin-bottom: var(--space-1);
	}
	.list {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
	button {
		box-sizing: border-box;
		display: inline-flex;
		align-items: center;
		gap: var(--space-025);
		min-height: var(--control-height);
		margin: 0;
		padding: 0 var(--space-1);
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--border-muted);
		border-radius: var(--radius-none);
		font: inherit;
		line-height: var(--type-button-line);
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
	button.warning:not(.active) {
		border-style: dashed;
		border-color: var(--state-error);
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

	/* ruled / sub: compact tabs on a rule (the creator's pages and the level below) */
	[data-variant='ruled'] > .row,
	[data-variant='sub'] > .row {
		border-bottom: var(--border-width-regular) solid var(--ink);
		padding-bottom: var(--space-025);
	}
	[data-variant='sub'] > .row {
		border-bottom-width: var(--border-width-hairline);
	}
	[data-variant='ruled'] .list,
	[data-variant='sub'] .list {
		gap: var(--space-025);
	}
	[data-variant='ruled'] button,
	[data-variant='sub'] button {
		padding: 0 var(--space-05);
	}

	/* grid: equal tabs in one frame, the open one inverted */
	/* `after` (e.g. the slice inspector's maximize button) sits right of the frame, never below it */
	[data-variant='grid'] > .row {
		flex-wrap: nowrap;
		align-items: start;
		gap: var(--space-05);
	}
	[data-variant='grid'] .list {
		flex: 1 1 auto;
		min-width: 0;
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(var(--tabs-min, 9rem), 1fr));
		gap: 0;
		border: var(--border-width-regular) solid var(--ink);
	}
	/* one line per tab (owner 2026-10-06: "Text boxes (0)" wrapped): the count is a small trailing number */
	[data-variant='grid'] button {
		justify-content: center;
		gap: var(--space-05);
		min-width: 0;
		padding: 0 var(--space-05);
		white-space: nowrap;
		background: transparent;
		border-color: transparent;
	}
	[data-variant='grid'] button:hover:not(:disabled):not(.active) {
		border-color: var(--border-muted);
	}
	[data-variant='grid'] button.active {
		background: var(--ink);
		border-color: var(--ink);
	}
	[data-variant='grid'] .label {
		overflow: hidden;
		text-overflow: ellipsis;
	}
	[data-variant='grid'] .count {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		opacity: 0.7;
	}
	/* phones (BACKLOG 10.50 e): every tab in ONE row – icon + count, the label stays the accessible name (+ the tooltip) */
	@media (max-width: 767px) {
		[data-variant='grid'] .list {
			grid-template-columns: repeat(auto-fit, minmax(0, 1fr));
		}
		[data-variant='grid'] .label {
			position: absolute;
			width: 1px;
			height: 1px;
			overflow: hidden;
			clip-path: inset(50%);
			white-space: nowrap;
		}
	}
</style>
