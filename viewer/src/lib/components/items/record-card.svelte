<script module lang="ts">
	import { fetchCredits } from '$lib/api.ts';

	// one request per toon and page (the credits change rarely; the API keeps its answer 60 s anyway)
	const creditsCache = new Map<string, ReturnType<typeof fetchCredits>>();
	function creditsOf(toon: string) {
		if (!creditsCache.has(toon)) {
			const request = fetchCredits(toon);
			request.catch(() => creditsCache.delete(toon));
			creditsCache.set(toon, request);
		}
		return creditsCache.get(toon)!;
	}
</script>

<script lang="ts">
	import { Icon, IconButton, Sheet } from '@scutoons/ui';
	import { resolveCredits, songSection, type ResolvedEntry } from '@scutoons/content';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import RecordDeck from './record-deck.svelte';
	import SeekBar from './seek-bar.svelte';
	import { formatTime } from './playlist.ts';

	/**
	 * The record player's expanded card (owner 2026-10-16: "clicking on the record player or track expands it to a larger
	 * card and shows the full name and credits for a song"): a bottom sheet over the page with the record large (the cover
	 * as its label) and "Turn the record" (its back), the song's full title, the album and n / N, the seek line,
	 * back / play-pause / next, the song's people from the toon's credits (`credits:<toon>`, its `song-…` section –
	 * `songSection`) and the album's songs to pick from. The sound lives in the bar (record-player.svelte): closing the
	 * card keeps it playing.
	 */
	let {
		open = $bindable(false),
		album,
		toon = null,
		cover,
		tracks,
		index,
		playing,
		changing,
		arm,
		time,
		length,
		side,
		failed,
		onturn,
		ontoggle,
		onseek,
		onback,
		onnext,
		onpick
	}: {
		open?: boolean;
		album: string;
		toon?: string | null;
		cover: string | null;
		tracks: { id?: string; title: string; duration?: number }[];
		index: number;
		playing: boolean;
		changing: boolean;
		arm: number;
		time: number;
		length: number;
		side: 'a' | 'b';
		failed: boolean;
		onturn: () => void;
		ontoggle: () => void;
		onseek: (seconds: number) => void;
		onback: () => void;
		onnext: () => void;
		onpick: (i: number) => void;
	} = $props();

	const track = $derived(tracks[index]);
	const count = $derived(tracks.length);

	// the song's people: loaded once the card opens; nothing shown while loading or when the song has no section
	let people = $state<ResolvedEntry[]>([]);
	$effect(() => {
		const song = track;
		if (!open || !toon || !song) return;
		let cancelled = false;
		const locale = getLocale();
		creditsOf(toon)
			.then(({ credits, people: registry, roles }) => {
				if (cancelled) return;
				const section = songSection(credits, song);
				people = section ? (resolveCredits({ ...credits, sections: [section] }, registry, roles, locale).sections[0]?.entries ?? []) : [];
			})
			.catch(() => !cancelled && (people = []));
		return () => (cancelled = true);
	});
</script>

<Sheet bind:open title={album} side="bottom" closeLabel={m['items.close']()}>
	<div class="card" data-testid="record-card">
		<div class="stage">
			<RecordDeck {cover} title={album} spinning={playing} {arm} lifted={changing} {side} size="min(15rem, 60vw)" />
			<button type="button" class="btn-secondary turn" aria-pressed={side === 'b'} onclick={onturn} data-testid="record-turn">
				<Icon name="refresh-cw" />
				{m['items.player_turn']()}
			</button>
		</div>

		<h3 class="song" data-testid="card-title">{track?.title}</h3>
		<p class="meta">{[album, count > 1 ? `${index + 1} / ${count}` : '', m['items.player_side']({ side: side.toUpperCase() })].filter(Boolean).join(' · ')}</p>

		{#if failed}
			<p class="note" role="alert">{m['items.release_failed']()}</p>
		{/if}
		<SeekBar {time} {length} label={m['items.player_seek']()} {onseek} testid="card-seek" />
		<div class="controls">
			{#if count > 1}
				<IconButton label={m['items.previous_track']()} onclick={onback} data-testid="card-prev"><Icon name="skip-back" /></IconButton>
			{/if}
			<IconButton class="play" label={playing ? m['items.player_pause']() : m['items.player_play']()} onclick={ontoggle} data-testid="card-play">
				<Icon name={playing ? 'pause' : 'play'} />
			</IconButton>
			{#if count > 1}
				<IconButton label={m['items.next_track']()} onclick={onnext} data-testid="card-next"><Icon name="skip-forward" /></IconButton>
			{/if}
		</div>

		{#if people.length}
			<section data-testid="card-credits">
				<h4>{m['items.player_credits']()}</h4>
				<ul class="people">
					{#each people as entry (entry.person.id)}
						<li><span class="name">{entry.person.name}</span>{#if entry.roles.length}<span class="roles">{entry.roles.join(', ')}</span>{/if}</li>
					{/each}
				</ul>
			</section>
		{/if}

		{#if count > 1}
			<section>
				<h4>{m['items.player_songs']()}</h4>
				<ol class="songs">
					{#each tracks as t, i (t.id ?? i)}
						<li>
							<button type="button" class="song-row" aria-current={i === index ? 'true' : undefined} onclick={() => onpick(i)} data-testid="card-song">
								<span class="n">{i + 1}</span>
								<span class="t">{t.title}</span>
								{#if t.duration}<span class="d">{formatTime(t.duration)}</span>{/if}
							</button>
						</li>
					{/each}
				</ol>
			</section>
		{/if}
	</div>
</Sheet>

<style>
	.card {
		max-width: 34rem;
		margin: 0 auto;
		display: flex;
		flex-direction: column;
		gap: var(--space-075);
	}

	.stage {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-05) 0;
	}

	.turn {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
	}

	.song {
		margin: 0;
		font-size: 1rem;
		font-weight: var(--font-weight-regular);
		line-height: 1.3;
		overflow-wrap: anywhere;
	}

	.meta,
	.note {
		margin: 0;
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.note {
		color: var(--ink);
	}

	.controls {
		display: flex;
		justify-content: center;
		align-items: center;
		gap: var(--space-1);
	}

	/* play / pause: the one filled button */
	.controls :global(.sc-icon-button.play) {
		background: var(--ink);
		color: var(--paper);
	}

	.controls :global(.sc-icon-button.play:hover) {
		background: var(--ink);
		outline: var(--border-width-regular) solid var(--primary-color);
	}

	section {
		border-top: var(--border-width-regular) solid var(--ink);
		padding-top: var(--space-075);
	}

	h4 {
		margin: 0 0 var(--space-05);
		font-size: 0.75rem;
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}

	ul,
	ol {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	.people li {
		display: flex;
		flex-wrap: wrap;
		gap: 0 var(--space-05);
		font-size: 0.75rem;
		line-height: 1.5;
	}

	.roles {
		color: var(--text-muted);
	}

	.song-row {
		box-sizing: border-box;
		width: 100%;
		min-height: var(--control-height);
		padding: 0 var(--space-05);
		display: flex;
		align-items: center;
		gap: var(--space-075);
		background: var(--paper);
		color: var(--ink);
		border: 0;
		border-radius: 0;
		font: inherit;
		font-size: 0.75rem;
		text-align: left;
		cursor: pointer;
	}

	.song-row:hover {
		background: var(--surface-hover);
	}

	.song-row[aria-current='true'] {
		background: var(--ink);
		color: var(--paper);
	}

	.n,
	.d {
		flex: none;
		font-variant-numeric: tabular-nums;
	}

	.t {
		flex: 1;
		min-width: 0;
	}
</style>
