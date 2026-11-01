<script lang="ts">
	// "Save as preset" on the browser deck (#270, research/INSTRUMENTS.md §2, §6): the performer picks the track, sees
	// what they changed against the preset it plays (the knobs that moved: from → to), names it and saves – a
	// derivative of that preset in their account (POST /api/rigs/presets; @scutoons/live-deck's preset.ts works out the
	// body, the server keeps only what differs from the parent's lineage). A modal sheet like the quality presets.
	import { Sheet, Button } from '@scutoons/ui';
	import { deriveBody, knobLabel, presetChanges, saveTracks, type RigPlayed, type DeriveBody } from '@scutoons/live-deck';
	import { m } from '$lib/paraglide/messages.js';

	let {
		open = $bindable(false),
		rig,
		values,
		locale = 'en',
		onsave
	}: {
		open?: boolean;
		/** the rig as played now */
		rig: RigPlayed;
		/** the knobs' values now (the deck's) */
		values: Record<string, number>;
		locale?: string;
		/** stores it; resolves with '' or why not */
		onsave: (body: DeriveBody) => Promise<string>;
	} = $props();

	const tracks = $derived(saveTracks(rig));
	let track = $state('');
	let name = $state('');
	let working = $state(false);
	let error = $state('');
	// the first track that can be saved, until the performer picks one
	const current = $derived(tracks.some((t) => t.track === track) ? track : (tracks[0]?.track ?? ''));
	const changes = $derived(current ? presetChanges(rig, current, values) : []);
	const parent = $derived(tracks.find((t) => t.track === current)?.parent ?? '');
	const fmt = (v: number) => (Math.abs(v) >= 100 ? v.toFixed(0) : Math.abs(v) >= 10 ? v.toFixed(1) : v.toFixed(2));

	async function save(event: SubmitEvent) {
		event.preventDefault();
		const body = deriveBody(rig, current, values, name, locale);
		if (!body || working) return;
		working = true;
		error = '';
		try {
			const why = await onsave(body);
			if (why) error = why;
			else {
				open = false;
				name = '';
			}
		} finally {
			working = false;
		}
	}
</script>

<Sheet bind:open side="bottom" title={m['live_deck.save_sheet']()} closeLabel={m['live_deck.save_close']()}>
	{#if tracks.length === 0}
		<p class="sc-empty">{m['live_deck.save_no_track']()}</p>
	{:else}
		<form class="save" onsubmit={save} data-testid="deck-save-sheet">
			<label for="save-track">{m['live_deck.save_track']()}</label>
			<select id="save-track" value={current} onchange={(e) => (track = e.currentTarget.value)}>
				{#each tracks as t (t.track)}
					<option value={t.track}>{t.track} · {t.parent}</option>
				{/each}
			</select>
			<label for="save-name">{m['live_deck.save_name']()}</label>
			<input id="save-name" bind:value={name} maxlength="100" placeholder={m['live_deck.save_name_placeholder']()} autocomplete="off" />
			<p class="what">{m['live_deck.save_changes']()}</p>
			{#if changes.length === 0}
				<p class="line">{m['live_deck.save_nothing']()}</p>
			{:else}
				<ul class="changes" data-testid="deck-save-changes">
					{#each changes as c (c.path)}
						<li><span class="knob">{knobLabel(c.knob, locale)}</span> <code>{c.path}</code> <span class="from">{fmt(c.from)}</span> → <b>{fmt(c.to)}</b></li>
					{/each}
				</ul>
			{/if}
			{#if error}<p class="line error" role="alert">{m['live_deck.save_failed']({ reason: error })}</p>{/if}
			<div class="row">
				<Button size="sm" variant="clear" onclick={() => (open = false)}>{m['live_deck.save_close']()}</Button>
				<Button size="sm" type="submit" disabled={working || !parent} data-testid="deck-save-do">{working ? m['live_deck.save_saving']() : m['live_deck.save_do']()}</Button>
			</div>
		</form>
	{/if}
</Sheet>

<style>
	.save {
		display: grid;
		gap: var(--space-025);
		font-size: 0.75rem;
	}
	/* the sheet is paper (the page behind it ink): everything from currentColor */
	label,
	.what {
		margin: var(--space-05) 0 0;
		font-size: 0.625rem;
		text-transform: uppercase;
		color: color-mix(in srgb, currentColor 60%, transparent);
	}
	select,
	input {
		height: var(--control-height);
		padding: 0 var(--space-05);
		border: var(--border-width-regular) solid color-mix(in srgb, currentColor 40%, transparent);
		border-radius: 0;
		background: none;
		color: inherit;
		font: inherit;
	}
	.changes {
		margin: 0;
		padding: 0;
		list-style: none;
		display: grid;
		gap: var(--space-025);
	}
	.changes li {
		display: flex;
		gap: var(--space-05);
		align-items: baseline;
		min-height: 1.5rem;
	}
	.knob {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	code,
	.from {
		color: color-mix(in srgb, currentColor 60%, transparent);
	}
	b {
		font-weight: var(--font-weight-regular);
	}
	.line {
		margin: 0;
		font-size: 0.625rem;
	}
	.error {
		color: var(--state-warning);
	}
	.row {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-05);
		margin-top: var(--space-05);
	}
</style>
