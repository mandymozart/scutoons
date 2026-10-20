<script lang="ts">
	// A card on the variants board (#125; VARIANTS-CANVAS §3.3): the picture – the image, a video at its first frame, the
	// render's schematic, a take (no picture yet) – with the STATUS bullet top left (✓ · ? · ✕) and a video's length chip
	// bottom right; under it the caption (name + model; selected = inverted with its letter A–D); the record as its
	// tooltip. The current card is framed twice (its actions hang under it). An option of its band's listbox (a roving
	// tabindex – only the current card is in the tab order). A REJECTED card shows only in a look or with Show rejected
	// (else it is a strip): muted. Below 60 % zoom the caption hides. The file loads once the card comes near the view
	// (the member's folder, the work storage, a provider's result – StoryboardPage.board.fileOf). A VIDEO SCRUBS under
	// the pointer (x = time): its own <video> seeks there, a line marks the spot, the chip says the time; a CLICK while it
	// scrubs pins that moment (More ▾ "Still: the frame at …"). Nobody drags cards (§3.2).
	import { m } from '$lib/paraglide/messages.js';
	import { TOP, type BoardNode, type CardStatus } from '$lib/editor/storyboard-board-layout.ts';
	import { lengthText, near, scrubAt, secondsText } from '$lib/editor/storyboard-posters.ts';
	import type { Generation } from '$lib/editor/storyboard-generations.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import StatusBullet from '$lib/components/storyboard/status-bullet.svelte';

	let { sb, node, g, picture, caption, captions, title, label, status, letter, current, onpick }: { sb: StoryboardPage; node: BoardNode; g: Generation | null; picture: string | null; caption: [string, string]; captions: boolean; title: string; label: string; status: CardStatus; letter: string; current: boolean; onpick: (e: MouseEvent) => void } = $props();
	let seen = $state(false);
	let url = $state<string | null>(null);
	let missing = $state(false);
	$effect(() => {
		const gen = g;
		if (!seen || !gen) return;
		let alive = true;
		void sb.board.fileOf(gen).then((u) => {
			if (!alive) return;
			url = u;
			missing = !u;
		});
		return () => {
			alive = false;
		};
	});
	const video = $derived(g?.kind === 'video');
	let player = $state<HTMLVideoElement>();
	let pic = $state<HTMLElement>();
	let lengthMs = $state<number | null>(null);
	const ms = $derived(lengthMs ?? (g?.duration ? g.duration * 1000 : null));
	let hover = $state<number | null>(null);
	const live = $derived(video && !!url);
	const scrub = $derived(live && hover != null ? scrubAt(hover, ms) : null);
	const pinned = $derived(live && ms && sb.board.pin?.key === node.key ? Math.min(1, sb.board.pin.ms / ms) : null);
	const known = (v: HTMLVideoElement | undefined) => !!v && Number.isFinite(v.duration) && v.duration > 0;

	function pick(e: MouseEvent) {
		if (scrub && ms) sb.board.pin = { key: node.key, ms: Math.round(scrub.ms) };
		onpick(e);
	}
	function move(e: PointerEvent) {
		if (!live || e.pointerType === 'touch' || !pic) return;
		const r = pic.getBoundingClientRect();
		hover = r.width > 0 ? Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)) : 0;
		if (known(player)) player!.currentTime = hover * player!.duration;
	}
	function leave() {
		hover = null;
		if (known(player)) player!.currentTime = 0.001;
	}
	const none = $derived(node.source === 'take' ? m['storyboard.board_no_picture']() : missing ? (g?.machine && g.machine !== 'work' ? m['storyboard.gen_elsewhere']({ machine: g.machine }) : m['storyboard.board_no_file']()) : '');
</script>

<button type="button" class="card" class:current class:picked={!!letter} class:rejected={status === 'rejected'} data-key={node.key} data-control-exempt="a card" tabindex={current ? 0 : -1} role="option" aria-selected={current} aria-label={label} {title} style:left="{node.x}rem" style:top="{node.y + TOP}rem" style:width="{node.w}rem" onclick={pick} onpointermove={move} onpointerleave={leave} use:near={() => (seen = true)} data-testid="card-{node.key}">
	<span class="pic" bind:this={pic} style:width="{node.pw}rem" style:height="{node.h}rem">
		{#if picture}
			<img src={picture} alt="" draggable="false" />
		{:else if url && video}
			<!-- svelte-ignore a11y_media_has_caption -->
			<video bind:this={player} src="{url}#t=0.001" preload="metadata" muted playsinline onloadedmetadata={() => (lengthMs = known(player) ? player!.duration * 1000 : null)}></video>
		{:else if url}
			<img src={url} alt="" draggable="false" />
		{:else if none}
			<span class="none" class:up={node.pw < 4}>{none}</span>
		{/if}
		{#if scrub}
			<i class="line" style:left="{scrub.share * 100}%" aria-hidden="true"></i>
		{:else if pinned != null}
			<i class="line" style:left="{pinned * 100}%" aria-hidden="true"></i>
		{/if}
		{#if status}<span class="corner"><StatusBullet {status} /></span>{/if}
		{#if video && ms}<span class="sc-chip len">{scrub ? secondsText(scrub.ms) : lengthText(ms)}</span>{/if}
	</span>
	{#if captions}<span class="m"><span>{letter ? `${letter} · ` : ''}{caption[0]}</span><span class="muted">{caption[1]}</span></span>{/if}
</button>

<style>
	.card {
		position: absolute;
		display: block;
		box-sizing: border-box;
		min-height: 0;
		margin: 0;
		padding: 0;
		border: 0;
		background: none;
		color: var(--ink);
		text-align: left;
		font-size: var(--type-caption-size);
		cursor: pointer;
		scroll-margin: 3.5rem 1rem 4rem;
	}
	.pic {
		position: relative;
		display: block;
		box-sizing: border-box;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--ink);
		overflow: hidden;
	}
	.card.current .pic {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: var(--border-width-regular);
	}
	img,
	video {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: contain;
		background: var(--paper);
	}
	.none {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-025);
		text-align: center;
		color: var(--paper);
	}
	.none.up {
		writing-mode: vertical-rl;
		transform: rotate(180deg);
		white-space: nowrap;
	}
	.corner {
		position: absolute;
		left: 0;
		top: 0;
		display: flex;
	}
	.line {
		position: absolute;
		top: 0;
		bottom: 0;
		width: var(--border-width-regular);
		margin-left: calc(var(--border-width-regular) / -2);
		background: var(--paper);
		box-shadow: 0 0 0 var(--border-width-hairline) var(--ink);
		pointer-events: none;
	}
	.len {
		position: absolute;
		right: 0;
		bottom: 0;
		text-transform: none;
		letter-spacing: normal;
	}
	.m {
		display: block;
		margin-top: 0.25rem;
		line-height: 1.25;
	}
	.m > span {
		display: block;
		padding: 0 0.125rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.muted {
		color: var(--text-muted);
	}
	.card.picked .m {
		background: var(--ink);
		color: var(--paper);
	}
	.card.picked .muted {
		color: var(--paper);
	}
	.card.rejected .pic {
		border-color: var(--border-muted);
		background: var(--paper);
	}
	.card.rejected .pic > :is(img, video) {
		opacity: 0.4;
	}
	.card.rejected .m,
	.card.rejected .none {
		color: var(--text-muted);
	}
	.card.rejected.current .pic {
		outline-color: var(--ink);
	}
</style>
