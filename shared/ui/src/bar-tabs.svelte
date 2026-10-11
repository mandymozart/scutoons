<script lang="ts">
	import Icon from './icon.svelte';

	/**
	 * Phones (UI-CONCEPTS E.10, owner 2026-10-11): a page's sheet tabs as icons in the one bottom bar (ActionBar `tabs`).
	 * The label is the accessible name + tooltip; the open tab is underlined. A tap on the open tab calls `onchange` again
	 * (the page may fold its sheet).
	 */
	let {
		tabs,
		value,
		label,
		onchange
	}: {
		tabs: { id: string; icon: string; label: string; count?: number; testid?: string }[];
		value: string;
		label: string;
		onchange: (id: string) => void;
	} = $props();
</script>

<div class="bar-tabs" role="tablist" aria-label={label}>
	{#each tabs as t (t.id)}
		<button
			type="button"
			role="tab"
			class:on={value === t.id}
			aria-selected={value === t.id}
			aria-label={t.count ? `${t.label} (${t.count})` : t.label}
			title={t.label}
			onclick={() => onchange(t.id)}
			data-testid={t.testid ?? `bar-tab-${t.id}`}
		>
			<Icon name={t.icon} />{#if t.count}<span class="count">{t.count}</span>{/if}
		</button>
	{/each}
</div>

<style>
	.bar-tabs {
		display: flex;
		align-items: stretch;
		min-width: 0;
		flex: 1;
	}
	.bar-tabs button {
		all: unset;
		box-sizing: border-box;
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		flex: 1 1 0;
		min-width: 2.5rem;
		height: var(--control-height);
		border-bottom: 0.1875rem solid transparent;
		color: var(--ink);
		cursor: pointer;
	}
	.bar-tabs button.on {
		border-bottom-color: var(--ink);
	}
	.bar-tabs button:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	.count {
		position: absolute;
		top: 0.1rem;
		right: 0.1rem;
		font-size: var(--type-caption-size);
		line-height: 1;
	}
</style>
