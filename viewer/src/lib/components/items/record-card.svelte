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
	import { Icon, IconButton } from '@scutoons/ui';
	import PullPanel from '$lib/components/panel/pull-panel.svelte';
	import { resolveCredits, songSection, type ResolvedEntry } from '@scutoons/content';
	import { getLocale, localizeHref } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { MediaQuery } from 'svelte/reactivity';
	import { webglAvailable } from '@scutoons/media/model-view';
	import type { ModelSource } from '@scutoons/content';
	import RecordDeck from './record-deck.svelte';
	import RecordStage, { TABLE, type StageTable } from './record-stage.svelte';
	import SeekBar from './seek-bar.svelte';
	import { formatTime } from './playlist.ts';

	/**
	 * The record player's expanded card (owner 2026-10-16: "clicking on the record player or track expands it to a larger
	 * card and shows the full name and credits for a song"): the reader's dark pull panel (pull-panel.svelte – like the
	 * inventory: pulled down, Escape or a tap outside closes it; owner 2026-10-16: "black … exactly such a panel") over the page with the record large (the cover
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
		records = [],
		current = null,
		onplayrecord,
		table = TABLE,
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
		/** the reader's records with a 3D model (record-shelf.ts; a .glb or the shared record model's look) – the 3D stage,
		 *  a carousel of them beside the turntable, instead of the 2D deck */
		records?: { key: string; model: ModelSource; name: string }[];
		/** the key of the record that plays */
		current?: string | null;
		/** Play on the carousel's front record (another one than the playing), or it was carried onto the turntable */
		onplayrecord?: (key: string) => void;
		/** the table's surface (an experiment – record-stage.svelte TABLE) */
		table?: StageTable;
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
		void records;
		modelFailed = false;
	});
	const in3d = $derived(records.length > 0 && open && !reduced.current && !modelFailed && webglAvailable());
	// the carousel's front record (a swipe selects): another than the playing one offers to put it on
	let selected = $state<string | null>(null);
	// the stage's scene fills the part of the panel above the song and controls: the rest of the canvas (the table running
	// out at the bottom) is what `lift` keeps below it
	let backdropHeight = $state(0), heroHeight = $state(0), overHeight = $state(0);
	const lift = $derived(backdropHeight ? Math.min(0.85, Math.max(0, 1 - (heroHeight - overHeight) / backdropHeight)) : 0.4);
	const picked = $derived(selected && selected !== current ? records.find((r) => r.key === selected) : null);
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

<!-- above the player bar (z 2040), below the item overlays / claim sheet / notices -->
<PullPanel {open} onclose={() => (open = false)} label={album} layer={2045} testid="record-panel">
	<div class="card" class:in3d data-testid="record-card">
		<!-- the 3D stage is the whole panel's background (owner 2026-10-16: "the player is the brown of the background"): it
		     stays put while the card scrolls over it – the song, the controls, the songs and the credits lie on the lit
		     table; the top part shows the records and the turntable. The 2D deck (no WebGL, reduced motion) stays in the
		     flow above the song. -->
		{#if in3d}
			<div class="backdrop" bind:clientHeight={backdropHeight}>
				<RecordStage {records} {current} {table} onselect={(key) => (selected = key)} onplay={(key) => onplayrecord?.(key)} {playing} {arm} lifted={changing} side={side === 'B' ? 'b' : 'a'} label={album} {lift} {resetKey} placed={playing || time > 0} onfail={() => (modelFailed = true)} />
				{#if picked}
					<!-- the carousel's front record is another one: put it on (owner 2026-10-16: "when you press Play it is put on") -->
					<div class="pick" data-testid="record-pick">
						<span>{picked.name}</span>
						<button type="button" class="btn-secondary" onclick={() => onplayrecord?.(picked.key)} data-testid="record-pick-play">{m['items.play']()}</button>
					</div>
				{/if}
			</div>
		{/if}
		<div class="hero" class:in3d bind:clientHeight={heroHeight}>
			{#if !in3d}
				<div class="deck">
					<RecordDeck {cover} title={album} spinning={playing} {arm} lifted={changing} side={side === 'B' ? 'b' : 'a'} size="min(15rem, 60vw)" />
				</div>
			{/if}
			<div class="over" bind:clientHeight={overHeight}>
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

		<!-- the song's people under the songs, then all of the toon's credits on its page (owner 2026-10-16: "Credits unter die
		     Trackliste … Show All … mit einem Anker auf die Credits") -->
		{#if people.length || toon}
			<section data-testid="card-credits">
				<h4>{m['items.player_credits']()}</h4>
				{#if people.length}
					<ul class="people">
						{#each people as entry (entry.person.id)}
							<li><span class="name">{entry.person.name}</span>{#if entry.roles.length}<span class="roles">{entry.roles.join(', ')}</span>{/if}</li>
						{/each}
					</ul>
				{/if}
				{#if toon}
					<a class="all" href={localizeHref(`/${toon}#credits`)} onclick={() => (open = false)} data-testid="card-credits-all">{m['items.player_credits_all']()} →</a>
				{/if}
			</section>
		{/if}
	</div>
</PullPanel>

<style>
	/* the column: 34rem at most – the 3D stage alone goes over the card's whole width (owner 2026-10-16: "im Hintergrund
	   über das ganze card bis links und rechts") */
	.card {
		--column: 34rem;
		display: flex;
		flex-direction: column;
		gap: var(--space-075);
	}

	.card > section,
	.hero:not(.in3d) {
		box-sizing: border-box;
		width: 100%;
		max-width: var(--column);
		margin-inline: auto;
	}

	/* the selected record's name + Play, over the stage's top left (where the carousel stands) */
	.pick {
		position: absolute;
		z-index: 1;
		pointer-events: none;
		top: var(--space-1);
		left: max(var(--space-1), env(safe-area-inset-left));
		display: flex;
		align-items: center;
		gap: var(--space-075);
		max-width: calc(100% - 2 * var(--space-1));
	}

	.pick button {
		pointer-events: auto;
	}

	.pick span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
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

	/* 3D: the canvas behind the whole panel, out to its edges (its body's padding) and up to its header; sticky, so it
	   stays while the card scrolls over it; it takes no room in the flow (the negative bottom margin, the gap included) */
	.backdrop {
		position: sticky;
		z-index: 0;
		top: calc(-1 * var(--space-1));
		height: 85dvh;
		margin: calc(-1 * var(--space-1)) calc(-1 * var(--space-1) - env(safe-area-inset-right)) calc(-85dvh - var(--space-075)) calc(-1 * var(--space-1) - env(safe-area-inset-left));
	}

	/* the card's own content lies over the stage and scrolls as any content: a touch on its text, headings or lists is
	   the card's, never the stage's (owner 2026-10-16: a scroll that started on the title moved the stage) */
	.card.in3d > :not(.backdrop) {
		position: relative;
		z-index: 1;
		text-shadow: 0 0 0.5rem rgb(0 0 0 / 0.65);
	}

	/* the top part where nothing but the 3D scene is: touches go to the stage (the records, the turntable); the song +
	   controls at its foot are content again */
	.hero.in3d {
		height: min(32rem, 62vh);
		justify-content: flex-end;
		pointer-events: none;
	}

	.hero.in3d .over {
		pointer-events: auto;
		box-sizing: border-box;
		width: 100%;
		max-width: var(--column);
		margin-inline: auto;
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

	/* "All credits →": a small text link under the song's people */
	.all {
		display: inline-block;
		margin-top: var(--space-05);
		font-size: 0.75rem;
		color: var(--ink);
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

	/* on the table (3D): the rows lie on it, no black boxes */
	.card.in3d .song-row:not([aria-current='true']):not(:hover) {
		background: transparent;
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
