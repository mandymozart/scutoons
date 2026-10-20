<script lang="ts">
	// One side of Compare (#125; VARIANTS-CANVAS §3.4): over it one line – its LETTER (A–D, inverted as the selected
	// card's caption), its name (a side from another shot than the current one names its shot), its STATUS bullet, and on
	// A the reference mark; the PICTURE at the height Compare gives every side, in its own shot's aspect – an image stands,
	// a video is one of the clock's (no native controls, muted, released when the side goes; a file that can't be played
	// says so); under it: model · length · seed · cost, what it came from, the prompt (3 lines; each whole in its tooltip).
	import { Icon } from '@scutoons/ui';
	import { shotSvg } from '@scutoons/content/storyboard';
	import { m } from '$lib/paraglide/messages.js';
	import { statusOf, type BoardNode } from '$lib/editor/storyboard-board-layout.ts';
	import { captionOf, sideFacts, titleOf, type WordsAt } from '$lib/editor/storyboard-board-words.ts';
	import { SIDE_HEAD, SIDE_INFO } from '$lib/editor/storyboard-compare.ts';
	import type { CompareClock } from '$lib/editor/storyboard-compare-clock.svelte.ts';
	import { svgSource } from '$lib/editor/storyboard-shots.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import StatusBullet from '$lib/components/storyboard/status-bullet.svelte';

	let { sb, shot, node, letter, h, pw, w, other, clock }: { sb: StoryboardPage; shot: string; node: BoardNode; letter: string; h: number; pw: number; w: number; other: boolean; clock: CompareClock } = $props();
	const words = $derived<WordsAt>({ shot, generators: sb.gen.generators, rate: sb.gen.budget?.rate ?? null, local: sb.localFiles, all: sb.generations });
	const g = $derived(node.record ? (sb.generations.find((x) => x.id === node.record!.id) ?? null) : null);
	const name = $derived(captionOf(node, words)[0]);
	const status = $derived(statusOf(node.record));
	let url = $state<string | null>(null);
	let broken = $state(false);
	$effect(() => {
		const gen = g;
		url = null;
		broken = false;
		if (!gen) return;
		let alive = true;
		void sb.board.fileOf(gen).then((u) => alive && (url = u));
		return () => {
			alive = false;
		};
	});
	const render = $derived.by(() => {
		if (node.source !== 'render') return null;
		const e = sb.entries.find((x) => x.id === shot);
		return e ? svgSource(shotSvg(e.shot, e.scene).svg) : null;
	});
	const video = $derived(g?.kind === 'video' && !!url);
	const ms = $derived(g?.duration ? g.duration * 1000 : null);
	const facts = $derived(sideFacts(node, words, ms));
	const track = (el: HTMLVideoElement, fallback: number | null) => clock.attach(el, fallback);
</script>

<figure class="side" style:width="{w}rem" data-side={letter} data-key={node.key} data-testid="compare-side-{letter}">
	<div class="sl" style:height="{SIDE_HEAD}rem" title={titleOf(node, words)}>
		<span class="letter">{letter}</span>
		<span class="name">{other ? `${shot} · ` : ''}{name}</span>
		<StatusBullet {status} />
		<span class="grow"></span>
		{#if letter === 'A'}<span class="ref" title={m['storyboard.compare_reference']()}><Icon name="bookmark" size="1rem" /></span>{/if}
	</div>
	<div class="pic" style:width="{pw}rem" style:height="{h}rem">
		{#if broken}
			<span class="none">{m['storyboard.compare_cant_play']()}</span>
		{:else if video}
			<!-- svelte-ignore a11y_media_has_caption -->
			<video src={url} aria-label="{shot} · {name}" muted playsinline preload="auto" disablepictureinpicture use:track={ms} onerror={() => (broken = true)}></video>
		{:else if url || render}
			<img src={url ?? render} alt="{shot} · {name}" draggable="false" />
		{:else}
			<span class="none">{m['storyboard.board_no_picture']()}</span>
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
	.letter {
		flex: none;
		padding: 0 var(--space-025);
		background: var(--ink);
		color: var(--paper);
		line-height: 1.25rem;
	}
	.name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.grow {
		flex: 1;
	}
	.ref {
		display: inline-flex;
		flex: none;
	}
	@container side (max-width: 7.5rem) {
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
	.facts,
	.prompt {
		display: -webkit-box;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.part:not(:first-child) {
		white-space: nowrap;
	}
	.part:not(:first-child)::before {
		content: '·\a0';
	}
	@container side (max-width: 9.5rem) {
		.seed {
			display: none;
		}
	}
</style>
