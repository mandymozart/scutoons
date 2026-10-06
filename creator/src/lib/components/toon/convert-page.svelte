<script lang="ts">
	import { onMount } from 'svelte';
	import { ActionBar, confirmDialog } from '@scutoons/ui';
	import { entities, parseEntityKey, visibleDifferences, type ToonLanguages } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { fileLabel } from '$lib/labels.ts';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { convertToon, differenceLine, planToon, versionCount, type EntityConversion } from '$lib/editor/conversion.ts';

	/**
	 * The owner's Convert page (agents/NEUTRAL.md "Migration"): the toon's chapters, scenes and seasons still stored per
	 * language. The dry run runs on open (read only): per entity what is stored (published / draft) and what readers will
	 * see differently (folded). "Convert" (one confirmation) saves and publishes the language-neutral documents; a second
	 * run finds nothing left to do.
	 */
	let { toon, languages }: { toon: string; languages: ToonLanguages } = $props();

	const nav = getCreatorNav();

	let plan = $state<EntityConversion[] | null>(null);
	let busy = $state(false);
	let done = $state(0);
	let message = $state<string | null>(null);
	let error = $state<string | null>(null);
	const count = $derived(plan ? versionCount(plan) : 0);
	/** a scene with its chapter (scene names repeat across chapters) */
	function label(entity: string) {
		const ref = parseEntityKey(entity);
		return ref?.kind === 'scene' && ref.number !== undefined ? `${fileLabel(entities.chapter(toon, ref.number))} · ${fileLabel(entity)}` : fileLabel(entity);
	}

	async function dryRun() {
		error = null;
		try {
			plan = await planToon(toon, languages);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}
	onMount(dryRun);

	async function convert() {
		if (!plan?.length) return;
		const name = nav.toonName(toon);
		if (!(await confirmDialog({ title: m['creator.convert_confirm_title']({ toon: name }), message: m['creator.convert_confirm']({ count: plan.length, toon: name }), confirmLabel: m['creator.convert']() }))) return;
		busy = true;
		error = null;
		message = null;
		done = 0;
		try {
			const stored = await convertToon(plan, (n) => (done = n));
			message = m['creator.convert_done']({ count: stored });
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
			await dryRun();
		}
	}
</script>

<div class="convert-page">
	<main class="body" data-testid="convert-page">
		<h1>{m['creator.convert_title']()}</h1>
		<p class="hint">{m['creator.convert_intro']()}</p>
		{#if error}<p class="error" data-testid="convert-error">{error}</p>{/if}
		{#if message}<p class="sc-bar is-note" data-testid="convert-message">{message}</p>{/if}
		{#if plan === null}
			<p>{m['creator.convert_loading']()}</p>
		{:else if !plan.length}
			<p class="sc-empty" data-testid="convert-none">{m['creator.convert_none']()}</p>
		{:else}
			<ul class="entities" data-testid="convert-list">
				{#each plan as entry (entry.entity)}
					<li data-testid="convert-{entry.entity}">
						<span class="name">{label(entry.entity)}</span>
						<span class="sc-caption">{m['creator.convert_from']({ locales: entry.locales.join(', ') })}</span>
						{#each entry.steps as step (step.status)}
							{@const visible = visibleDifferences(step.report)}
							<div class="step">
								<span class="sc-chip">{step.status === 'published' ? m['creator.convert_published']() : m['creator.convert_draft']()}</span>
								{#if visible.length}
									<details>
										<summary>{m['creator.convert_differences']({ count: visible.length })}</summary>
										<ul class="differences">
											{#each visible as r, i (i)}<li>{differenceLine(r)}</li>{/each}
										</ul>
									</details>
								{:else}
									<span class="sc-caption">{m['creator.converted_no_differences']()}</span>
								{/if}
							</div>
						{/each}
					</li>
				{/each}
			</ul>
		{/if}
	</main>
	<ActionBar label={m['creator.page_actions']()} moreLabel={m['creator.more']()}>
		{#snippet start()}
			{#if busy && plan}<span class="hint">{done} / {plan.length}</span>{/if}
		{/snippet}
		{#snippet end()}
			<button disabled={busy || !count} onclick={convert} data-testid="convert">{m['creator.convert']()}</button>
		{/snippet}
	</ActionBar>
</div>

<style>
	.convert-page {
		display: flex;
		flex-direction: column;
		min-height: 100%;
	}
	.body {
		flex: 1;
		padding: var(--space-1);
		max-width: 56rem;
	}
	.entities {
		list-style: none;
		margin: var(--space-1) 0 0;
		padding: 0;
	}
	.entities > li {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: var(--space-05) var(--space-1);
		padding: var(--space-05) 0;
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.step {
		flex-basis: 100%;
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: var(--space-05);
	}
	.step details {
		flex: 1;
		min-width: 0;
	}
	.differences {
		max-height: 16rem;
		overflow: auto;
		margin: var(--space-05) 0 0;
		padding-left: var(--space-1);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		overflow-wrap: anywhere;
	}
	.error {
		color: var(--error);
	}
</style>
