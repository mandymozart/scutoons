<script lang="ts">
	import { Icon, IconButton, CloseIcon } from '@scutoons/ui';
	import type { AudioRequest } from '$lib/contexts/items-ui.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { armAngle, nextOnSide, previousOnSide, sideIndices, sideOf, songLength, turnTo, twoSided } from './playlist.ts';
	import RecordCard from './record-card.svelte';
	import type { StageTable } from './record-stage.svelte';
	import { warmModel } from './model-warmup.ts';
	import { shelfOf } from './record-shelf.ts';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { getItemsUiContext } from '$lib/contexts/items-ui.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import ItemIcon from './item-icon.svelte';
	import MarqueeText from './marquee-text.svelte';
	import SeekBar from './seek-bar.svelte';
	import { wakeLock } from '$lib/wake-lock.svelte.ts';

	/**
	 * The audio player at the bottom (owner 2026-10-14: "play the whole album – like a little playlist, or an animation on
	 * the record player"; 2026-10-16: "improve the player" after res.radio's): the album's songs one after another. A new
	 * album opens the card at once; closed, a compact bar – the cover, the song's title and "album · Side A · n / N"
	 * (a marquee when long), back / play-pause / next, close, the seek line below. A tap on the cover or the title opens the card (record-card.svelte: the record large,
	 * "Turn the record", the full title, the song's credits, the side's songs). Songs play on within their record side
	 * (owner 2026-10-16: tracks carry `side`, none = A); after the side's last song the side starts again; turning the
	 * record goes to the other side's first song (an album with one side only turns the picture).
	 * A vault track gets its signed address at the reader's media quality only when it starts (`request.sign` – the
	 * addresses live ten minutes, an album runs longer); one that ran out while paused is signed again where it stood.
	 * No native controls: the state follows the media's own events (a blocked autoplay shows ▶, never a false "playing").
	 * Reduced motion: the record stands still, the arm jumps (record-deck.svelte).
	 */
	let {
		request,
		icon = null,
		table,
		layout = 'a',
		onclose
	}: {
		request: AudioRequest;
		icon?: { src: string; name: string } | null;
		/** the 3D stage's table surface (the lab compares; else record-stage.svelte TABLE) */
		table?: StageTable;
		/** the mini bar's layout (the lab compares a / b; owner 2026-10-16) */
		layout?: 'a' | 'b';
		onclose: () => void;
	} = $props();

	let index = $state(0);
	let src = $state<string | null>(null);
	let failed = $state(false);
	let playing = $state(false);
	/** between two songs: the arm is lifted for a moment */
	let changing = $state(false);
	let time = $state(0);
	let mediaDuration = $state(0);
	let audio = $state<HTMLAudioElement>();
	let expanded = $state(false);
	/** a one-sided album: "Turn the record" only turns the picture */
	let flipped = $state(false);
	/** a signed address that ran out: signed again once, playing on from here */
	let resumeAt = 0;
	let retried = false;

	const count = $derived(request.tracks.length);
	const track = $derived(request.tracks[Math.min(index, count - 1)]);
	const cover = $derived(request.cover ? asset(request.cover) : (icon?.src ?? null));
	const length = $derived(songLength(mediaDuration, track?.duration));
	const sided = $derived(twoSided(request.tracks));
	const side = $derived(sided ? sideOf(track) : flipped ? 'B' : 'A');
	/** the side's songs (album indices) and where this one is among them */
	const onSide = $derived(sided ? sideIndices(request.tracks, side) : request.tracks.map((_, i) => i));
	const position = $derived(onSide.indexOf(index) + 1);

	// a new album starts at its first song (side A's first when the album has sides) – with the card open (owner
	// 2026-10-16: "Play … open the expanded 3D card right away"; the mini bar shows once the reader closes it)
	$effect(() => {
		const tracks = request.tracks;
		index = twoSided(tracks) ? (sideIndices(tracks, 'A')[0] ?? 0) : 0;
		flipped = false;
		expanded = true;
	});

	function address(i: number): Promise<string> {
		const raw = request.tracks[i]?.src ?? '';
		return request.sign ? request.sign(i) : Promise.resolve(/^https?:/.test(raw) ? raw : asset(raw));
	}

	// the song's address: signed just before it plays
	$effect(() => {
		const i = index;
		void request;
		let cancelled = false;
		failed = false;
		retried = false;
		resumeAt = 0;
		src = null;
		mediaDuration = 0;
		time = 0;
		address(i)
			.then((url) => !cancelled && (src = url))
			.catch(() => !cancelled && (failed = true));
		return () => (cancelled = true);
	});

	/** the file failed – a signed address that ran out is asked for again once, then it plays on where it stood */
	function errored() {
		if (!src) return;
		if (retried || !request.sign) {
			failed = true;
			playing = false;
			return;
		}
		retried = true;
		resumeAt = time;
		const i = index;
		address(i)
			.then((url) => index === i && (src = url))
			.catch(() => index === i && (failed = true));
	}

	function loaded() {
		if (resumeAt > 0 && audio) {
			audio.currentTime = resumeAt;
			resumeAt = 0;
			void audio.play().catch(() => {});
		}
	}

	let gap: ReturnType<typeof setTimeout> | null = null;
	function go(i: number) {
		if (gap) clearTimeout(gap);
		gap = null;
		changing = false;
		// the same song again (a one-song album looping): its address stays, so start it over by hand
		if (i === index && audio) {
			audio.currentTime = 0;
			void audio.play().catch(() => {});
		}
		index = i;
	}

	/** a song ended: lift the arm a moment, then the side's next one (after the side's last: its first again) */
	function ended() {
		const next = nextOnSide(request.tracks, index);
		playing = false;
		if (next === null) return;
		changing = true;
		gap = setTimeout(() => go(next), 1200);
	}

	/** back: the song again a few seconds in, else the one before */
	function back() {
		const i = previousOnSide(request.tracks, index, audio?.currentTime ?? 0);
		if (i === index && audio) audio.currentTime = 0;
		else go(i);
	}

	const next = () => go(nextOnSide(request.tracks, index) ?? index);

	/** "Turn the record": the other side's first song; one side only: the picture */
	function turn() {
		const to = turnTo(request.tracks, index);
		if (to === null) flipped = !flipped;
		else go(to);
	}

	function toggle() {
		if (!audio || !src) return;
		if (audio.paused) void audio.play().catch(() => {});
		else audio.pause();
	}

	/** the tonearm on the 3D stage: set down on the record = play from there (its place → the song's position), back at
	 *  rest = pause (owner 2026-10-16: "the tonearm is draggable") */
	function armSet(to: { play: true; progress: number } | { play: false }) {
		if (!audio || !src) return;
		if (!to.play) return audio.pause();
		if (length) seek(to.progress * length);
		if (audio.paused) void audio.play().catch(() => {});
	}

	function seek(seconds: number) {
		if (!audio) return;
		audio.currentTime = seconds;
		time = seconds;
	}

	$effect(() => () => {
		if (gap) clearTimeout(gap);
	});

	// the screen stays on while a record plays (PWA phase 4, lib/wake-lock.svelte.ts); paused / closed → it may sleep
	$effect(() => (playing || changing ? wakeLock.hold('record-player') : undefined));

	// the bar is up: the card may open any moment – its 3D stage's three.js and model load ahead in idle time
	$effect(() => warmModel(request.model));

	// the reader's records for the card's carousel (record-shelf.ts); the one that plays is always among them
	const inventory = getInventoryContext();
	const membership = getMembershipContext();
	const ui = getItemsUiContext();
	const shelf = $derived(shelfOf(inventory, membership.isValid));
	const currentKey = $derived(request.entry ? String(request.entry.id) : 'playing');
	const records = $derived.by(() => {
		const list = shelf.map((r) => ({ key: r.key, model: r.model, name: r.name }));
		if (request.model && !list.some((r) => r.key === currentKey)) list.unshift({ key: currentKey, model: request.model, name: request.title });
		return list;
	});
	/** Play on another record in the carousel: its album plays (the stage takes the one on the platter off first) */
	function playRecord(key: string) {
		const next = shelf.find((r) => r.key === key);
		if (next && key !== currentKey) ui.playAlbum(next.request);
	}

	const arm = $derived(armAngle(playing || changing, changing ? 0 : length ? time / length : 0));
	/** the bar's second line: the album, the side, n / N */
	const meta = $derived([request.title, sided ? m['items.player_side']({ side }) : '', onSide.length > 1 ? `${position} / ${onSide.length}` : ''].filter(Boolean).join(' · '));
	const heading = $derived(m['items.now_playing']({ title: [track?.title, request.title].filter(Boolean).join(' · ') }));
</script>

<!-- data-panel-ignore: a tap on the bar is not "outside" an open panel (closing the bar left the inventory open – owner 2026-10-16) -->
<!-- the mini bar (owner 2026-10-16): two rows – the cover, the song and its album / side, the controls as bare symbols; the
     timeline over the whole width below (variant a, like res.radio's bar); variant b keeps the controls beside the timeline.
     Hidden while the card is open – the card is the player then; the sound plays on (the audio element stays). -->
<div class="record-player bar-{layout}" class:hidden={expanded} data-testid="audio-player" data-panel-ignore>
	<button type="button" class="thumb" onclick={() => (expanded = true)} aria-label={m['items.player_open']()} title={m['items.player_open']()} data-testid="audio-open">
		<ItemIcon srcs={[icon?.src ?? '', request.cover ?? '']} name={request.title} />
	</button>
	<button type="button" class="line" onclick={() => (expanded = true)} title={heading} data-testid="audio-expand">
		<MarqueeText text={track?.title || request.title} testid="audio-title" />
		<MarqueeText text={meta} testid="audio-count" />
	</button>
	<div class="buttons">
		{#if onSide.length > 1}
			<button type="button" class="bare prev" aria-label={m['items.previous_track']()} title={m['items.previous_track']()} onclick={back} data-testid="audio-prev"><Icon name="skip-back" /></button>
		{/if}
		<button type="button" class="bare play" aria-label={playing ? m['items.player_pause']() : m['items.player_play']()} title={playing ? m['items.player_pause']() : m['items.player_play']()} onclick={toggle} disabled={!src || failed} data-testid="audio-play">
			<Icon name={playing ? 'pause' : 'play'} />
		</button>
		{#if onSide.length > 1}
			<button type="button" class="bare next" aria-label={m['items.next_track']()} title={m['items.next_track']()} onclick={next} data-testid="audio-next"><Icon name="skip-forward" /></button>
		{/if}
	</div>
	<button type="button" class="bare close" onclick={onclose} aria-label={m['items.close']()} title={m['items.close']()} data-testid="audio-close"><CloseIcon size="1rem" /></button>
	<div class="timeline">
		{#if failed}
			<p class="note" role="alert">{m['items.release_failed']()}</p>
		{:else}
			<SeekBar {time} {length} label={m['items.player_seek']()} onseek={seek} testid="audio-seek" />
		{/if}
	</div>
	<!-- svelte-ignore a11y_media_has_caption -->
	<audio
		bind:this={audio}
		src={src ?? undefined}
		preload="metadata"
		autoplay
		bind:currentTime={time}
		bind:duration={mediaDuration}
		onplay={() => (playing = true)}
		onpause={() => (playing = false)}
		onended={ended}
		onerror={errored}
		onloadedmetadata={loaded}
		data-testid="audio-element"
	></audio>
</div>

<RecordCard
	bind:open={expanded}
	album={request.title}
	toon={request.toon ?? null}
	{cover}
	{records}
	current={currentKey}
	onplayrecord={playRecord}
	onarm={armSet}
	{table}
	tracks={request.tracks}
	{index}
	{onSide}
	{sided}
	{playing}
	{changing}
	{arm}
	{time}
	{length}
	{side}
	{failed}
	onturn={turn}
	ontoggle={toggle}
	onseek={seek}
	onback={back}
	onnext={next}
	onpick={go}
/>

<style>
	.record-player {
		position: fixed;
		z-index: 2040;
		left: var(--space-05);
		right: var(--space-05);
		bottom: calc(var(--space-05) + env(safe-area-inset-bottom));
		max-width: 34rem;
		margin: 0 auto;
		box-sizing: border-box;
		display: grid;
		grid-template-columns: 2.75rem minmax(0, 1fr) auto auto;
		grid-template-areas:
			'thumb line buttons close'
			'time time time time';
		align-items: center;
		column-gap: var(--space-05);
		row-gap: 0;
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		padding: var(--space-05) var(--space-05) var(--space-025);
	}

	/* variant b (like res.radio's: play | song | close): play and close beside the song, back / next flank the timeline */
	.record-player.bar-b {
		grid-template-areas:
			'thumb line play close'
			'prev time time next';
	}

	.bar-b .buttons {
		display: contents;
	}

	.bar-b .prev {
		grid-area: prev;
	}

	.bar-b .play {
		grid-area: play;
	}

	.bar-b .next {
		grid-area: next;
	}

	.hidden {
		display: none;
	}

	.thumb {
		grid-area: thumb;
		width: 2.75rem;
		height: 2.75rem;
		overflow: hidden;
	}

	.line {
		grid-area: line;
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
		min-width: 0;
		text-align: left;
		font-size: 0.875rem;
		line-height: 1.25;
	}

	.line :global([data-testid='audio-count']) {
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	/* the cover and the song open the card: frameless buttons around them */
	.thumb,
	.line,
	.bare {
		margin: 0;
		padding: 0;
		border: 0;
		border-radius: 0;
		background: none;
		color: inherit;
		font: inherit;
		cursor: pointer;
	}

	.thumb:focus-visible,
	.line:focus-visible,
	.bare:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: 0.125rem;
	}

	.buttons {
		grid-area: buttons;
		display: flex;
	}

	/* the controls: bare symbols (owner 2026-10-16: "icon-only buttons without borders"), each a full touch target */
	.bare {
		box-sizing: border-box;
		width: var(--control-height);
		height: var(--control-height);
		display: inline-grid;
		place-items: center;
		flex: none;
	}

	.bare:hover:not(:disabled) {
		color: var(--primary-color);
	}

	.bare:disabled {
		opacity: 0.4;
		cursor: default;
	}

	.close {
		grid-area: close;
	}

	.timeline {
		grid-area: time;
		min-width: 0;
	}

	audio {
		display: none;
	}

	.note {
		margin: 0;
		font-size: 0.75rem;
	}
</style>
