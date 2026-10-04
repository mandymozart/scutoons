<script lang="ts">
	import { confirmRemove } from '$lib/editor/ask.ts';
	import ReorderButtons from '$lib/components/controls/reorder-buttons.svelte';
	import { Icon } from '@scutoons/ui';
	import { tick } from 'svelte';
	import { CREDIT_SECTION_KINDS, sectionName, type CreditSection, type CreditSectionKind, type ToonCredits } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { creditsOf, edit, move, newSection } from '$lib/editor/credits.ts';
	import { m } from '$lib/paraglide/messages.js';
	import EntryList from '$lib/components/credits/entry-list.svelte';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';

	/** The ordered sections of the credits: add / reorder / remove sections and edit their people. */
	let {
		credits,
		people,
		roles,
		languages,
		onperson
	}: { credits: ContentDoc; people: ContentDoc; roles: ContentDoc; languages: string[]; onperson?: (id: string) => void } = $props();

	const file = $derived(creditsOf(credits));
	const sections = $derived(file?.sections ?? []);
	let addKind = $state<CreditSectionKind>('song');
	let root = $state<HTMLElement>();

	const change = (fn: (file: ToonCredits) => void, key?: string) => edit<ToonCredits>(credits, fn, key);
	const kindLabel = (kind: CreditSectionKind) => m[`creator.credits_kind_${kind}`]();

	async function moveSection(from: number, to: number, focus = false) {
		if (to < 0 || to >= sections.length) return;
		change((f) => move(f.sections, from, to));
		if (focus) {
			await tick();
			root?.querySelectorAll<HTMLElement>('[data-section-handle]')[to]?.focus();
		}
	}
	function add() {
		change((f) => f.sections.push(newSection(addKind, f, languages)));
	}
	async function remove(i: number, section: CreditSection) {
		const count = section.entries.length + (section.kind === 'toon' ? section.lead.length : 0);
		if (count && !(await confirmRemove(m['creator.credits_remove_section_confirm']({ name: sectionName(section, languages[0]), count })))) return;
		change((f) => f.sections.splice(i, 1));
	}
</script>

<div class="sections" bind:this={root} data-testid="credits-sections">
	{#each sections as section, i (section.id)}
		<section class="section" data-testid="credits-section" data-kind={section.kind} data-section={section.id}>
			<header>
				<button
					type="button"
					class="handle"
					data-section-handle
					aria-label={m['creator.credits_move']({ name: sectionName(section, languages[0]) })}
					title={m['creator.credits_move_hint']()}
					onkeydown={(e) => {
						if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
							e.preventDefault();
							moveSection(i, i + (e.key === 'ArrowUp' ? -1 : 1), true);
						}
					}}>⋮⋮</button
				>
				<span class="kind">{kindLabel(section.kind)}</span>
				{#if section.kind === 'season' || section.kind === 'chapter'}
					<label class="num">
						#
						<input
							type="number"
							min="1"
							value={section.number}
							oninput={(e) => change((f) => ((f.sections[i] as any).number = Number(e.currentTarget.value) || 1), `num:${section.id}`)}
						/>
					</label>
				{:else}
					<span class="title">{sectionName(section, languages[0])}</span>
				{/if}
				<span class="spacer"></span>
				<ReorderButtons index={i} count={sections.length} onmove={(from, to) => moveSection(from, to)} variant="ghost mini" labels={{ up: m['creator.credits_up'](), down: m['creator.credits_down']() }} />
				<button type="button" class="ghost mini" onclick={() => remove(i, section)} data-testid="remove-section">{m['creator.credits_remove']()}</button>
			</header>

			{#if section.kind === 'song' || section.kind === 'performance'}
				<LocaleMapField value={section.title} {languages} label={m['creator.credits_title']()} required onchange={(v) => change((f) => ((f.sections[i] as any).title = v), `title:${section.id}`)} />
			{/if}
			{#if section.kind === 'performance'}
				<div class="row">
					<LocaleMapField value={section.venue} {languages} label={m['creator.credits_venue']()} onchange={(v) => change((f) => ((f.sections[i] as any).venue = v), `venue:${section.id}`)} />
					<label class="date">
						<span>{m['creator.credits_date']()}</span>
						<input type="text" value={section.date ?? ''} placeholder="11/2021" oninput={(e) => change((f) => ((f.sections[i] as any).date = e.currentTarget.value), `date:${section.id}`)} />
					</label>
				</div>
			{/if}

			{#if section.kind === 'toon'}
				<h4>{m['creator.credits_lead']()}</h4>
				<EntryList {credits} {people} {roles} {languages} {onperson} path={['sections', i, 'lead']} testid="lead-{section.id}" />
				<h4>{m['creator.credits_entries']()}</h4>
			{/if}
			<EntryList {credits} {people} {roles} {languages} {onperson} path={['sections', i, 'entries']} testid="entries-{section.id}" />
		</section>
	{/each}

	<div class="add">
		<select bind:value={addKind} aria-label={m['creator.credits_section_kind']()} data-testid="add-section-kind">
			{#each CREDIT_SECTION_KINDS as kind (kind)}<option value={kind}>{kindLabel(kind)}</option>{/each}
		</select>
		<button type="button" class="secondary" onclick={add} data-testid="add-section"><Icon name="plus" /> {m['creator.credits_add_section']()}</button>
	</div>
</div>

<style>
	.sections {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.section {
		border: 2px solid var(--ink);
		padding: 0.5rem 0.75rem 0.75rem;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-wrap: wrap;
	}
	.handle {
		all: unset;
		display: inline-flex;
		align-items: center;
		min-height: var(--control-height);
		cursor: grab;
		color: var(--text-muted);
		letter-spacing: -2px;
		padding: 0 0.25rem;
	}
	.handle:focus-visible {
		outline: 2px solid var(--ink);
	}
	.kind {
		font-size: var(--type-caption-size);
		letter-spacing: var(--tracking-label);
		text-transform: uppercase;
		background: var(--ink);
		color: var(--paper);
		padding: 0 0.4rem;
	}
	.num input {
		width: 4rem;
	}
	.spacer {
		flex: 1;
	}
	.mini {
		font-size: var(--type-caption-size);
		padding: 0 0.4rem;
	}
	h4 {
		margin: 0.25rem 0 0;
	}
	.row {
		display: flex;
		gap: 0.75rem;
		align-items: flex-end;
		flex-wrap: wrap;
	}
	.row > :global(:first-child) {
		flex: 1;
	}
	.date {
		display: flex;
		flex-direction: column;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.add {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}
</style>
