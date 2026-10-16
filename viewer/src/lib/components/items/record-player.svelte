<script lang="ts">
	import { Icon, IconButton, CloseIcon } from '@scutoons/ui';
	import type { AudioRequest } from '$lib/contexts/items-ui.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { armAngle, nextOnSide, previousOnSide, sideIndices, sideOf, songLength, turnTo, twoSided } from './playlist.ts';
	import RecordCard from './record-card.svelte';
	import RecordDeck from './record-deck.svelte';
	import SeekBar from './seek-bar.svelte';

	/**
	 * The audio player at the bottom (owner 2026-10-14: "play the whole album – like a little playlist, or an animation on
	 * the record player"; 2026-10-16: "improve the player" after res.radio's): the album's songs one after another, a
	 * compact bar – the turntable, the song's title and "Side A · n / N", the seek line with the time and the length, back /
	 * play-pause / next, close. A tap on the record or the title opens the card (record-card.svelte: the record large,
	 * "Turn the record", the full title, the song's credits, the side's songs). Songs play on within their record side
	 * (owner 2026-10-16: tracks carry `side`, none = A); after the side's last song the side starts again; turning the
	 * record goes to the other side's first song (an album with one side only turns the picture).
	 * A vault track gets its signed address at the reader's media quality only when it starts (`request.sign` – the
	 * addresses live ten minutes, an album runs longer); one that ran out while paused is signed again where it stood.
	 * No native controls: the state follows the media's own events (a blocked autoplay shows ▶, never a false "playing").
	 * Reduced motion: the record stands still, the arm jumps (record-deck.svelte).
	 */
	let { request, icon = null, onclose }: { request: AudioRequest; icon?: { src: string; name: string } | null; onclose: () => void } = $props();

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

	// a new album starts at its first song (side A's first when the album has sides)
	$effect(() => {
		const tracks = request.tracks;
		index = twoSided(tracks) ? (sideIndices(tracks, 'A')[0] ?? 0) : 0;
		flipped = false;
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

	function seek(seconds: number) {
		if (!audio) return;
		audio.currentTime = seconds;
		time = seconds;
	}

	$effect(() => () => {
		if (gap) clearTimeout(gap);
	});

	const arm = $derived(armAngle(playing || changing, changing ? 0 : length ? time / length : 0));
	const heading = $derived(m['items.now_playing']({ title: [track?.title, request.title].filter(Boolean).join(' · ') }));
</script>

<div class="record-player" data-testid="audio-player">
	<button type="button" class="deck-button" onclick={() => (expanded = true)} aria-label={m['items.player_open']()} title={m['items.player_open']()} data-testid="audio-open">
		<RecordDeck {cover} title={request.title} spinning={playing} {arm} lifted={changing} side={side === 'B' ? 'b' : 'a'} />
	</button>
	<div class="track">
		<button type="button" class="line" onclick={() => (expanded = true)} title={heading} data-testid="audio-expand">
			<span class="track-title" data-testid="audio-title">{heading}</span>
			{#if sided || onSide.length > 1}<span class="count" data-testid="audio-count">{sided ? `${m['items.player_side']({ side })} · ` : ''}{position} / {onSide.length}</span>{/if}
		</button>
		{#if failed}
			<p class="note" role="alert">{m['items.release_failed']()}</p>
		{:else}
			<SeekBar {time} {length} label={m['items.player_seek']()} onseek={seek} testid="audio-seek" />
		{/if}
	</div>
	<div class="buttons">
		{#if onSide.length > 1}
			<IconButton class="prev" label={m['items.previous_track']()} onclick={back} data-testid="audio-prev"><Icon name="skip-back" /></IconButton>
		{/if}
		<IconButton label={playing ? m['items.player_pause']() : m['items.player_play']()} onclick={toggle} disabled={!src || failed} data-testid="audio-play">
			<Icon name={playing ? 'pause' : 'play'} />
		</IconButton>
		{#if onSide.length > 1}
			<IconButton label={m['items.next_track']()} onclick={next} data-testid="audio-next"><Icon name="skip-forward" /></IconButton>
		{/if}
	</div>
	<button class="close" onclick={onclose} aria-label={m['items.close']()}><CloseIcon size="1rem" /></button>
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
		bottom: var(--space-05);
		max-width: 34rem;
		margin: 0 auto;
		display: flex;
		align-items: center;
		gap: var(--space-05);
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		padding: var(--space-025) var(--space-05);
	}

	/* the turntable and the title open the card: frameless buttons around them */
	.deck-button,
	.line {
		margin: 0;
		padding: 0;
		border: 0;
		border-radius: 0;
		background: none;
		color: inherit;
		font: inherit;
		cursor: pointer;
	}

	.deck-button {
		flex: none;
		display: block;
	}

	.deck-button:focus-visible,
	.line:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: 0.125rem;
	}

	.track {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
	}

	.line {
		display: flex;
		gap: var(--space-05);
		font-size: 0.75rem;
		text-align: left;
		min-width: 0;
	}

	.track-title {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.count {
		flex: none;
		font-variant-numeric: tabular-nums;
	}

	audio {
		display: none;
	}

	.note {
		margin: 0;
		font-size: 0.75rem;
	}

	.buttons {
		display: flex;
		flex: none;
		gap: var(--space-025);
	}

	/* a phone keeps play and next in the bar; back is in the card */
	@media (max-width: 479.98px) {
		.buttons :global(.prev) {
			display: none;
		}
	}

	.close {
		box-sizing: border-box;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		border-radius: 50%;
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		display: inline-grid;
		place-items: center;
		flex: none;
	}

	.close:hover {
		background: var(--primary-color);
	}
</style>
