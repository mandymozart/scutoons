<script>
	// One side of Compare (agents/research/VARIANTS-CANVAS.md §3.4; the mockup research/studio-layout/variants-canvas.html):
	// over it one line – its LETTER (A–D, inverted as the selected card's caption on the board), its name (a side from
	// another shot than the current one names its shot: "sb-25-1 · g4" – in a narrow side "25-1·g4" and no reference
	// mark (the letter A says it), narrower still the shot's part ellipsized, the card's id stays whole), its STATUS
	// bullet (StatusBullet – the cards'), and on A the reference mark; the PICTURE at the height Compare gives every
	// side, in its own shot's aspect (another ratio letterboxed, as on the cards) – an image stands, a video is one of
	// the clock's (compare-clock.svelte.js: no native controls, muted, released when the side goes; a file that can't be
	// played says so); under it: model · length · seed · cost (each part whole, wrapping between parts – up to 3 lines; a
	// narrow side leaves the seed out), what it came from (where it hangs on the board – its main input – and the other
	// parents), the prompt (3 lines); each line's whole text in its tooltip. The record as the head's tooltip
	// (board.svelte.js titleOf).
	import { Icon } from '@scutoons/ui';
	import { statusOf } from '../board-layout.js';
	import { captionOf, titleOf } from '../board.svelte.js';
	import { mediaOf } from '../card-media.js';
	import { SIDE_HEAD, SIDE_INFO } from '../compare.js';
	import { sideFacts } from '../compare.svelte.js';
	import { ch, studio } from '../studio.svelte.js';
	import StatusBullet from './StatusBullet.svelte';

	/** @type {{ side: { shot: string, key: string, node: any, takes: Map<string, any> }, letter: string, h: number, pw: number, w: number, other: boolean, clock: { attach: (el: HTMLVideoElement, ms: number|null) => { destroy: () => void } } }} */
	let { side, letter, h, pw, w, other, clock } = $props();
	const node = $derived(side.node);
	const media = $derived(mediaOf(node, { ch: ch(), shot: side.shot, takes: side.takes, v: studio.version }));
	const name = $derived(captionOf(node, media)[0]);
	/** "sb-25-1" → ["sb-", "25-1"] – the prefix goes first in a narrow side */
	const shotParts = $derived(/^([a-z]+-)?(.*)$/i.exec(side.shot).slice(1));
	const status = $derived(statusOf(node.record));
	const facts = $derived(sideFacts(node, media));
	const video = $derived(media.video && !!media.url && !media.missing);
	/** the render: clean → the saved frame when that is missing; a video that can't be played */
	let failed = $state([]);
	const src = $derived([media.url, media.fallback].find((u) => u && !failed.includes(u)) ?? null);
	const broken = $derived(video && failed.includes(media.url));
	/** the clock's action (a member call can't be a directive's name) */
	const track = (el, ms) => clock.attach(el, ms);
	const reference = $derived(letter === 'A');
</script>

<figure class="side" style:width="{w}rem" data-side={letter} data-key={side.key}>
	<div class="sl" style:height="{SIDE_HEAD}rem" title={titleOf(node, side.shot, media)}>
		<span class="letter">{letter}</span>
		<span class="name">{#if other}<span class="shot"><span class="pre">{shotParts[0] ?? ''}</span>{shotParts[1]}<span class="sep"></span></span>{/if}<span class="id">{name}</span></span>
		<StatusBullet {status} />
		<span class="grow"></span>
		{#if reference}<span class="ref" title="A – the reference: it stays while a click on a card (in any band) replaces the last side"><Icon name="bookmark" size="1rem" /></span>{/if}
	</div>
	<div class="pic" style:width="{pw}rem" style:height="{h}rem">
		{#if media.missing}
			<span class="none">file gone</span>
		{:else if broken}
			<span class="none">the file can't be played</span>
		{:else if video}
			<!-- one of the clock's: no controls of its own (one Play, one scrubber – CompareTransport) -->
			<!-- svelte-ignore a11y_media_has_caption -->
			<video src={media.url} aria-label="{side.shot} · {name}" muted playsinline preload="auto" disablepictureinpicture use:track={media.ms ?? null} onerror={() => (failed = [...failed, media.url])}></video>
		{:else if src}
			<img {src} alt="{side.shot} · {name}" draggable="false" onerror={() => (failed = [...failed, src])} />
		{:else}
			<span class="none">no picture yet</span>
		{/if}
	</div>
	<figcaption style:height="{SIDE_INFO}rem">
		<span class="facts" title={facts.line}>{#each facts.parts as p, i}{#if i}{' '}{/if}<span class="part" class:seed={p.seed}>{p.text}</span>{/each}</span>
		<span class="line muted" title={facts.from}>{facts.from}</span>
		{#if facts.prompt}<span class="prompt" title={facts.prompt}>{facts.prompt}</span>{/if}
	</figcaption>
</figure>

<style>
	.side {
		container: side / inline-size;
		display: flex;
		flex: none;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
		margin: 0;
	}
	.sl {
		display: flex;
		align-items: center;
		gap: var(--space-025);
		min-width: 0;
		font-size: var(--type-caption-size);
		white-space: nowrap;
	}
	/* the letter as on the board: the selected card's caption is inverted with it */
	.letter {
		flex: none;
		padding: 0 var(--space-025);
		background: var(--ink);
		color: var(--paper);
		line-height: 1.25rem;
	}
	.name {
		display: flex;
		min-width: 0;
		overflow: hidden;
	}
	.name > span {
		white-space: nowrap;
	}
	.shot {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.id {
		flex: none;
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.sep::before {
		content: '\a0·\a0';
	}
	/* a narrow side: "25-1 · g1" (the whole id in the line's tooltip); narrower "25-1·g1", no reference mark (A says it) */
	@container side (max-width: 9.5rem) {
		.pre {
			display: none;
		}
	}
	.grow {
		flex: 1;
	}
	.ref {
		display: inline-flex;
		flex: none;
	}
	@container side (max-width: 7.5rem) {
		.sl {
			gap: 0.125rem;
		}
		.letter {
			padding: 0 0.125rem;
		}
		.sep::before {
			content: '·';
		}
		.ref {
			display: none;
		}
	}
	.pic {
		position: relative;
		box-sizing: border-box;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--ink);
		overflow: hidden;
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
		font-size: var(--type-caption-size);
	}
	figcaption {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
		overflow: hidden;
		font-size: var(--type-caption-size);
		line-height: 1.3;
	}
	.line {
		min-height: 1.3em;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.muted {
		color: var(--text-muted);
	}
	/* model · length · seed · cost: up to three lines in a narrow column; the prompt: three, the whole in its tooltip */
	.facts,
	.prompt {
		display: -webkit-box;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	/* each part whole (a line breaks between parts – the model's words may wrap), the dot before all but the first */
	.part:not(:first-child) {
		white-space: nowrap;
	}
	.part:not(:first-child)::before {
		content: '·\a0';
	}
	/* a narrow side: the seed goes (the cost stays in view – the whole line in the tooltip) */
	@container side (max-width: 9.5rem) {
		.seed {
			display: none;
		}
	}
</style>
