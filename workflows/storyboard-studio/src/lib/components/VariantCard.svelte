<script>
	// A card on the variants board (agents/research/VARIANTS-CANVAS.md §3.3): the picture – the image, a video's poster (its
	// first frame, posters.js – loaded once the card comes near the view) – with the STATUS bullet top left (✓ chosen ·
	// ? review · ✕ rejected – one square, icon only: no chip text, no side bar; owner 2026-10-20: "the icon is enough";
	// none = nothing) and a video's length chip bottom right; under it the caption (id + model; selected = inverted with
	// its letter A–D); the record as its tooltip. The current card is framed twice (its actions hang under it – Band). A
	// button as an option of its band's listbox (aria-selected = the current card; the A–D letter is in its name): a click
	// = the current card, Ctrl / ⌘ + click = the selection (Board); a roving tabindex – only the current card is in the
	// tab order, the board's keys move it. Another ratio than the shot's is letterboxed in the picture.
	import { Icon } from '@scutoons/ui';
	import { TOP } from '../board-layout.js';
	import { lengthOf } from '../board.svelte.js';
	import { devicePx, near, posterOf } from '../posters.js';

	/** @type {{ node: any, media: { url: string|null, fallback?: string|null, video: boolean, ms?: number|null, missing?: boolean }, caption: string[], title: string, label: string, status: string|null, letter: string, current: boolean, onpick: (e: MouseEvent) => void }} */
	let { node, media, caption, title, label, status, letter, current, onpick } = $props();
	const ICONS = { chosen: 'check', review: 'question', rejected: 'x' };
	const WORDS = { chosen: 'Chosen', review: 'Review – to look at again', rejected: 'Rejected' };
	/** near the view: the picture loads */
	let seen = $state(false);
	/** a video's first frame (a JPEG) – or, without one, the <video> at its first frame */
	let poster = $state(null);
	let still = $state(false);
	/** the image urls that failed (the render: clean → the saved frame → nothing) */
	let failed = $state([]);
	const src = $derived([media.url, media.fallback].find((u) => u && !failed.includes(u)) ?? null);
	/** a narrow picture (1:3 and narrower) writes its "no picture" upwards, as the Rail's label – a word wouldn't fit across */
	const narrow = $derived(node.pw < 4);

	$effect(() => {
		const url = media.url;
		if (!seen || !url || !media.video || media.missing) return;
		let alive = true;
		posterOf(url, devicePx(node.h)).then((p) => {
			if (alive) [poster, still] = [p, !p];
		});
		return () => {
			alive = false;
		};
	});
</script>

<button type="button" class="card" class:current class:picked={!!letter} data-key={node.key} data-control-exempt="a card" tabindex={current ? 0 : -1} role="option" aria-selected={current} aria-label={label} {title} style:left="{node.x}rem" style:top="{node.y + TOP}rem" style:width="{node.w}rem" onclick={onpick} use:near={() => (seen = true)}>
	<span class="pic" style:width="{node.pw}rem" style:height="{node.h}rem">
		{#if seen}
			{#if media.missing}
				<span class="none" class:up={narrow}>file gone</span>
			{:else if media.video && media.url}
				{#if poster}
					<img src={poster} alt="" />
				{:else if still}
					<!-- svelte-ignore a11y_media_has_caption -->
					<video src="{media.url}#t=0.001" preload="metadata" muted playsinline></video>
				{/if}
			{:else if src}
				<img {src} alt="" onerror={() => (failed = [...failed, src])} />
			{:else}
				<span class="none" class:up={narrow}>no picture yet</span>
			{/if}
		{/if}
		{#if status}<span class="stat {status}" title={WORDS[status]}><Icon name={ICONS[status]} size="0.95rem" /></span>{/if}
		{#if media.video && media.ms}<span class="sc-chip len">{lengthOf(media.ms)}</span>{/if}
	</span>
	<span class="m"><span>{letter ? `${letter} · ` : ''}{caption[0]}</span><span class="muted">{caption[1]}</span></span>
</button>

<style>
	/* a card, not a button row (the studio's buttons.css gives every button the control's shape – undone here) */
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
		/* the board's keys bring a card into view below the sticky band head */
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
	/* the current card (the last clicked): a second ink frame around the picture */
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
	/* the status bullet: filled on the badge tokens, the glyph ink (published 11.56 : 1, warning 7.15 : 1); rejected
	   recedes – a muted frame, a muted ✕ */
	.stat {
		position: absolute;
		left: 0;
		top: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		width: 1.25rem;
		height: 1.25rem;
		color: var(--ink);
	}
	.stat.chosen {
		background: var(--badge-published);
	}
	.stat.review {
		background: var(--badge-warning);
	}
	.stat.rejected {
		background: var(--paper);
		border: var(--border-width-regular) solid var(--border-muted);
		color: var(--text-muted);
	}
	/* data in a chip keeps its case (DESIGN "Badges with data … stay in their case") */
	.len {
		position: absolute;
		right: 0;
		bottom: 0;
		text-transform: none;
		letter-spacing: normal;
	}
	/* the caption: id (+ its letter) and the model, one line each */
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
</style>
