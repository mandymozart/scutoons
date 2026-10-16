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
	import { MediaQuery } from 'svelte/reactivity';
	import { webglAvailable } from '@scutoons/media/model-view';
	import RecordDeck from './record-deck.svelte';
	import RecordStage from './record-stage.svelte';
	import SeekBar from './seek-bar.svelte';
	import { formatTime } from './playlist.ts';

	/**
	 * The record player's expanded card (owner 2026-10-16: "clicking on the record player or track expands it to a larger
	 * card and shows the full name and credits for a song"): a bottom sheet over the page with the record large (the cover
	 * as its label on side A, a paper label on side B) and "Turn the record" (the other side's songs, owner 2026-10-16),
	 * the song's full title, the album, the side and n / N, the seek line, back / play-pause / next, the song's people from
	 * the toon's credits (`credits:<toon>`, its `song-…` section – `songSection`) and the side's songs to pick from. With the item's 3D model (FEATURES 10.102: the LP's sleeve with the record inside) the
	 * record is a 3D stage (record-stage.svelte): on play it comes out of the sleeve onto a turntable and turns; without
	 * WebGL, with reduced motion or a model that fails: the 2D deck. The sound lives in the bar (record-player.svelte): closing the
	 * card keeps it playing.
	 */
	let {
		open = $bindable(false),
		album,
		toon = null,
		cover,
		model = null,
		tracks,
		index,
		onSide,
		sided,
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
		/** the item's 3D model (CDN URL) – the 3D stage instead of the 2D deck */
		model?: string | null;
		tracks: { id?: string; title: string; duration?: number }[];
		index: number;
		/** the side's songs (album indices) */
		onSide: number[];
		/** the album has two sides */
		sided: boolean;
		playing: boolean;
		changing: boolean;
		arm: number;
		time: number;
		length: number;
		side: 'A' | 'B';
		failed: boolean;
		onturn: () => void;
		ontoggle: () => void;
		onseek: (seconds: number) => void;
		onback: () => void;
		onnext: () => void;
		onpick: (i: number) => void;
	} = $props();

	const track = $derived(tracks[index]);
	const reduced = new MediaQuery('prefers-reduced-motion: reduce');
	let modelFailed = $state(false);
	/** bumped on a pick in the song list: the 3D stage eases the reader's pivot / zoom back */
	let resetKey = $state(0);
	$effect(() => {
		void model;
		modelFailed = false;
	});
	const in3d = $derived(!!model && open && !reduced.current && !modelFailed && webglAvailable());
	const count = $derived(onSide.length);
	const position = $derived(onSide.indexOf(index) + 1);

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
		<!-- the 3D stage is the top's background, the song and the controls lie over its lower part (owner 2026-10-16:
		     "covering the entire background of the card so it doesn't get cut off … the top part on top the player
		     controls"); the 2D deck (no WebGL, reduced motion) stays in the flow above them -->
		<div class="hero" class:in3d={in3d && model}>
			{#if in3d && model}
				<RecordStage url={model} {playing} {arm} lifted={changing} side={side === 'B' ? 'b' : 'a'} label={album} lift={0.22} {resetKey} onfail={() => (modelFailed = true)} />
			{:else}
				<div class="deck">
					<RecordDeck {cover} title={album} spinning={playing} {arm} lifted={changing} side={side === 'B' ? 'b' : 'a'} size="min(15rem, 60vw)" />
				</div>
			{/if}
			<div class="over">
				<h3 class="song" data-testid="card-title">{track?.title}</h3>
				<p class="meta">{[album, sided || side === 'B' ? m['items.player_side']({ side }) : '', count > 1 ? `${position} / ${count}` : ''].filter(Boolean).join(' · ')}</p>

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
					<!-- the record's side as an A / B toggle among the controls (owner 2026-10-16: "part of the player's controls … a
					     smaller button", "maybe we can do an AB, two buttons, side A or side B, a toggle"): the other side turns it -->
					<div class="sides" role="group" aria-label={m['items.player_turn']()} data-testid="record-turn">
						{#each ['A', 'B'] as const as s (s)}
							<button type="button" class:on={side === s} aria-pressed={side === s} title={m['items.player_side']({ side: s })} onclick={() => side !== s && onturn()} data-testid="record-side-{s}">{s}</button>
						{/each}
					</div>
				</div>
			</div>
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
					{#each onSide as i, n (i)}
						{@const t = tracks[i]}
						<li>
							<button type="button" class="song-row" aria-current={i === index ? 'true' : undefined} onclick={() => { resetKey++; onpick(i); }} data-testid="card-song">
								<span class="n">{sided ? side : ''}{n + 1}</span>
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

	.hero {
		position: relative;
		display: flex;
		flex-direction: column;
		gap: var(--space-075);
	}

	.deck {
		display: flex;
		justify-content: center;
		padding: var(--space-05) 0;
	}

	.over {
		display: flex;
		flex-direction: column;
		gap: var(--space-075);
	}

	/* 3D: the canvas fills the top; the turn button in its corner, the song + controls over its lower part, on a paper
	   fade so they stay readable over the scene */
	.hero.in3d {
		height: min(30rem, 58vh);
		justify-content: flex-end;
	}

	.hero.in3d .over {
		position: relative;
		z-index: 1;
		padding-top: 3rem;
		background: linear-gradient(to bottom, transparent, var(--paper) 2.75rem);
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

	/* the record's side: two square letters in one frame, the side it plays inverted */
	.sides {
		display: inline-flex;
		border: var(--border-width-regular) solid var(--ink);
	}

	.sides button {
		box-sizing: border-box;
		min-width: var(--control-height);
		height: var(--control-height);
		padding: 0 var(--space-05);
		background: var(--paper);
		color: var(--ink);
		border: 0;
		border-radius: 0;
		font: inherit;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.sides button + button {
		border-left: var(--border-width-regular) solid var(--ink);
	}

	.sides button.on {
		background: var(--ink);
		color: var(--paper);
		cursor: default;
	}

	.sides button:not(.on):hover {
		background: var(--surface-hover);
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
