<script lang="ts">
	import { Badge, Icon } from '@scutoons/ui';
	import {
		AUTHORED_VIEW,
		CHARACTER_IMAGE_VARIANTS,
		DEFAULT_MOOD,
		MOOD_ID,
		characterImage,
		conventionImagePath,
		lookImage,
		lookMoods,
		lookPortraitMoods,
		moodLabel,
		sortedMoods,
		type AssetRequest,
		type AssetRequests,
		type CharacterLook,
		type MoodVocabulary,
		type RegistryCharacter
	} from '@scutoons/content';
	import { imageExists, recheckImages, isKnownMissing } from '$lib/editor/characters.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import AssetDialog from './asset-dialog.svelte';

	/**
	 * The moods of one look (of one persona) – the portrait is the authored view: a live CDN preview per mood, ✓ found /
	 * a red "not on the CDN" badge, and a dashed "missing – add image" cell that opens the asset dialog (set the path, prefilled with
	 * the convention path `characters/<character>[-<persona>]/<character>[-<persona>][_<look>]_<view>-<mood>.webp` (delivered as PNG; the characters import writes the WebP), or leave a
	 * note = an asset request). The avatar is derived from the portrait (a small indicator); full is unused (only with
	 * "all views"). Empty cells show faded what the reader shows instead (`resolveExpression`).
	 */
	let {
		character,
		persona,
		look,
		moods,
		requests,
		locale,
		onset,
		onrequest,
		onaddmood
	}: {
		character: RegistryCharacter;
		persona: string;
		look: CharacterLook;
		moods: MoodVocabulary;
		requests: AssetRequests | null;
		/** language of the mood labels */
		locale: string;
		onset: (mood: string, view: string, path: string) => void;
		onrequest: (mood: string, view: string, note: string) => void;
		/** a mood for this look (added to the vocabulary when it is new) */
		onaddmood: (mood: string, label: string) => void;
	} = $props();

	/** CDN check per path: true found, false missing, undefined pending */
	let status = $state<Record<string, boolean | undefined>>({});
	let round = $state(0);
	let showStandard = $state(true);
	let allViews = $state(false);
	let newMood = $state('');
	let dialog = $state<{ mood: string; view: string } | null>(null);
	let dialogOpen = $state(false);

	const standard = $derived(sortedMoods(moods).filter((x) => x.standard).map((x) => x.id));
	const rowMoods = $derived.by(() => {
		const own = lookMoods(look, moods);
		if (!showStandard) return own;
		const order = sortedMoods(moods).map((x) => x.id);
		return [...new Set([...own, ...standard])].sort((a, b) => {
			if (a === DEFAULT_MOOD) return -1;
			if (b === DEFAULT_MOOD) return 1;
			const ia = order.indexOf(a);
			const ib = order.indexOf(b);
			return (ia < 0 ? Infinity : ia) - (ib < 0 ? Infinity : ib);
		});
	});
	const views = $derived(allViews ? [...CHARACTER_IMAGE_VARIANTS] : [AUTHORED_VIEW]);
	const openRequests = $derived((requests?.requests ?? []).filter((r) => r.status === 'open' && r.character === character.id && r.persona === persona && r.look === look.id));
	const requestFor = (mood: string, view: string): AssetRequest | undefined => openRequests.find((r) => r.mood === mood && r.view === view);
	const portraitMoods = $derived(lookPortraitMoods(look));
	const hasNeutral = $derived(!!lookImage(look, DEFAULT_MOOD, AUTHORED_VIEW));
	const standardDone = $derived(standard.filter((x) => portraitMoods.includes(x)).length);

	function cell(mood: string, view: string) {
		const path = lookImage(look, mood, view);
		const convention = conventionImagePath(character.id, view, mood, look.id, persona);
		const fallback = path ? '' : characterImage(character, { persona, look: look.id, mood }, view, { isMissing: isKnownMissing });
		return { path, convention, fallback };
	}

	$effect(() => {
		void round;
		const paths = new Set<string>();
		for (const mood of rowMoods)
			for (const view of CHARACTER_IMAGE_VARIANTS) {
				const c = cell(mood, view);
				paths.add(c.path || c.convention);
				if (c.fallback) paths.add(c.fallback);
			}
		for (const path of paths) {
			if (path in status) continue;
			status[path] = undefined;
			imageExists(path).then((ok) => (status[path] = ok));
		}
	});

	function openDialog(mood: string, view: string) {
		dialog = { mood, view };
		dialogOpen = true;
	}

	function addMood() {
		const id = newMood.trim().toLowerCase().replace(/\s+/g, '-');
		if (!MOOD_ID.test(id)) return;
		onaddmood(id, newMood.trim());
		newMood = '';
	}

	function recheck() {
		recheckImages();
		status = {};
		round++;
	}
	const stateOf = (c: ReturnType<typeof cell>) =>
		c.path ? (status[c.path] === true ? 'found' : status[c.path] === false ? 'missing' : 'pending') : status[c.convention] === true ? 'available' : 'empty';
</script>

<section class="expressions" data-testid="expression-grid">
	<div class="bar">
		<span class="summary" data-testid="expression-summary">
			{m['creator.characters_moods_count']({ count: portraitMoods.length })} ·
			<Badge tone={hasNeutral ? 'published' : 'warning'}>{DEFAULT_MOOD}</Badge>
			· <span class="hint">{m['creator.characters_standard_set']({ done: standardDone, total: standard.length })}</span>
		</span>
		<span class="actions">
			<label class="check"><input type="checkbox" bind:checked={showStandard} data-testid="show-standard" /> {m['creator.characters_show_standard']()}</label>
			<label class="check"><input type="checkbox" bind:checked={allViews} data-testid="all-views" /> {m['creator.characters_all_views']()}</label>
			<button type="button" class="ghost" onclick={recheck}>{m['creator.characters_recheck']()}</button>
		</span>
	</div>
	<div class="grid" class:all={allViews} role="grid">
		{#each rowMoods as mood (mood)}
			<div class="mood-row" role="row" data-mood={mood}>
				<span class="mood" role="rowheader">
					{moodLabel(mood, locale, moods)}
					{#if moodLabel(mood, locale, moods) !== mood}<span class="hint">{mood}</span>{/if}
					{#if standard.includes(mood)}<span class="std" title={m['creator.characters_standard']()}>★</span>{/if}
				</span>
				{#each views as view (view)}
					{@const c = cell(mood, view)}
					{@const state = stateOf(c)}
					{@const request = requestFor(mood, view)}
					{@const avatar = view === AUTHORED_VIEW && !allViews ? cell(mood, 'avatar') : null}
					<button
						type="button"
						role="gridcell"
						class="cell state-{state}"
						data-control-exempt="picture cell"
						class:requested={!!request}
						title={c.path || c.convention}
						onclick={() => openDialog(mood, view)}
						data-testid="cell-{mood}-{view}"
						data-state={state}
					>
						<span class="thumb">
							{#if state === 'found'}
								<img src={asset(c.path)} alt="{mood} {view}" loading="lazy" />
							{:else if state === 'available'}
								<img class="faded" src={asset(c.convention)} alt="" loading="lazy" />
							{:else if c.fallback && status[c.fallback] === true}
								<img class="faded" src={asset(c.fallback)} alt="" loading="lazy" />
							{/if}
						</span>
						<span class="label">
							{#if state === 'found'}<Badge tone="published">{m['creator.characters_found']()}</Badge>{:else if state === 'missing'}<Badge tone="error">{m['creator.characters_missing_file']()}</Badge>{:else if state === 'available'}◌ {m['creator.characters_available']()}{:else if state === 'pending'}…{:else}{m['creator.characters_missing_add']()}{/if}
							{#if allViews}<span class="hint">{view}</span>{/if}
							{#if avatar}
								<span class="derived" title={m['creator.characters_avatar_derived']()}>avatar {#if !avatar.path}–{:else if status[avatar.path] === false}<Badge tone="error">{m['creator.characters_missing_file']()}</Badge>{:else}✓{/if}</span>
							{/if}
							{#if request}<span class="req" title={request.note ?? ''}><Icon name="mail" /> {m['creator.characters_requested_short']()}</span>{/if}
						</span>
					</button>
				{/each}
			</div>
		{/each}
	</div>
	<p class="add-mood">
		<input type="text" list="mood-vocabulary" bind:value={newMood} placeholder={m['creator.characters_new_mood']()} onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), addMood())} data-testid="new-mood" />
		<datalist id="mood-vocabulary">{#each sortedMoods(moods).filter((x) => !rowMoods.includes(x.id)) as x (x.id)}<option value={x.id}>{moodLabel(x.id, locale, moods)}</option>{/each}</datalist>
		<button type="button" class="ghost" onclick={addMood} disabled={!MOOD_ID.test(newMood.trim().toLowerCase().replace(/\s+/g, '-'))}><Icon name="plus" /> {m['creator.characters_add_mood']()}</button>
	</p>
</section>

{#if dialog}
	{@const c = cell(dialog.mood, dialog.view)}
	{#key `${dialog.mood}/${dialog.view}`}
		<AssetDialog
			bind:open={dialogOpen}
			title="{character.id} · {persona} · {look.id} · {moodLabel(dialog.mood, locale, moods)} ({dialog.view})"
			path={c.path}
			convention={c.convention}
			request={requestFor(dialog.mood, dialog.view)}
			onset={(path) => dialog && onset(dialog.mood, dialog.view, path)}
			onrequest={(note) => dialog && onrequest(dialog.mood, dialog.view, note)}
		/>
	{/key}
{/if}

<style>
	.bar {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
		align-items: center;
		margin-bottom: 0.4rem;
	}
	.ok {
		color: var(--ink);
	}
	.bad {
		color: var(--error);
	}
	.actions {
		display: flex;
		gap: 0.6rem;
		align-items: center;
		margin-left: auto;
		flex-wrap: wrap;
	}
	.check {
		font-size: var(--type-caption-size);
		display: flex;
		gap: 0.25rem;
		align-items: center;
	}
	.grid { /* grid-ok: the expression thumbnails, not fields */
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(11rem, 1fr));
		gap: 4px;
	}
	.grid.all {
		grid-template-columns: 1fr;
	}
	.mood-row {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.grid.all .mood-row {
		display: grid;
		grid-template-columns: 8rem repeat(3, minmax(0, 1fr));
		gap: 4px;
	}
	.mood {
		font-weight: var(--font-weight-regular);
		display: flex;
		gap: 0.3rem;
		align-items: baseline;
	}
	.std {
		font-size: var(--type-caption-size);
		color: var(--muted);
	}
	.cell {
		all: unset;
		box-sizing: border-box;
		cursor: pointer;
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		padding: 0.25rem;
		border: 1px solid var(--border-subtle);
		background: var(--paper);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.cell:focus-visible {
		outline: 2px solid var(--ink);
	}
	.cell.state-found {
		border-color: var(--ink);
	}
	.cell.state-missing {
		border: 1px solid var(--error);
		border-left: 0.5rem solid var(--state-error);
		color: var(--error);
	}
	.cell.state-empty,
	.cell.state-available {
		border: 1px dashed var(--ink);
	}
	.cell.state-empty {
		color: var(--muted);
	}
	.thumb {
		aspect-ratio: 1;
		width: 100%;
		background: var(--surface-hover);
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}
	.grid.all .thumb {
		aspect-ratio: auto;
		height: 4.5rem;
	}
	.thumb img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}
	.faded {
		opacity: 0.3;
	}
	.label {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
		align-items: center;
	}
	.derived {
		margin-left: auto;
		font-size: var(--type-caption-size);
		color: var(--muted);
	}
	.req {
		font-size: var(--type-caption-size);
		background: var(--badge-info);
		color: var(--badge-info-text);
		padding: 0 0.25rem;
	}
	.add-mood {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
		align-items: center;
		margin-top: 0.5rem;
	}
	.add-mood input {
		width: 12rem;
	}
</style>
