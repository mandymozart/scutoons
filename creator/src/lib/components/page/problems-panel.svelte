<script lang="ts">
	import { CloseIcon } from '@scutoons/ui';
	import { fileLabel } from '$lib/labels.ts';
	import type { Problem } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	let {
		problems,
		ongoto,
		onclose
	}: { problems: Problem[]; ongoto?: (problem: Problem) => void; onclose?: () => void } = $props();

	let showInfo = $state(false);
	const visible = $derived(problems.filter((p) => showInfo || p.level !== 'info'));
	const counts = $derived({
		error: problems.filter((p) => p.level === 'error').length,
		warning: problems.filter((p) => p.level === 'warning').length,
		info: problems.filter((p) => p.level === 'info').length
	});
</script>

<div class="problems" data-testid="problems-panel">
	<header>
		<h2>{m['creator.problems']()} · {counts.error} errors · {counts.warning} warnings</h2>
		{#if onclose}<button class="ghost" onclick={onclose} aria-label="Close"><CloseIcon /></button>{/if}
	</header>
	<label class="toggle"><input type="checkbox" bind:checked={showInfo} /> show {counts.info} notes</label>
	{#if !visible.length}
		<p class="hint">{m['creator.no_problems']()}</p>
	{/if}
	<ul>
		{#each visible as p, i (i)}
			<li class="level-{p.level}">
				<button type="button" class="ghost" onclick={() => ongoto?.(p)}>
					<span class="where">{fileLabel(p.file)}{p.locale ? ` · ${p.locale}` : ''} · {p.path.join('.') || '/'}</span>
					<span class="msg">{p.message}</span>
				</button>
			</li>
		{/each}
	</ul>
</div>

<style>
	header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		border-bottom: 2px solid var(--ink);
	}
	h2 {
		margin: 0.25rem 0;
	}
	.toggle {
		display: flex;
		gap: 0.3rem;
		align-items: center;
		margin: 0.3rem 0;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	li {
		border-left: 3px solid var(--border-muted);
		margin-bottom: 0.2rem;
	}
	li.level-error {
		border-left-color: var(--error);
	}
	li.level-warning {
		border-left-color: var(--warn);
	}
	li button {
		display: flex;
		flex-direction: column;
		text-align: left;
		width: 100%;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.where {
		color: var(--muted);
		font-size: var(--type-caption-size);
		word-break: break-all;
	}
</style>
