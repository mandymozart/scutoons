<script lang="ts">
	// One band of the variants board (#125; VARIANTS-CANVAS §3.2, Q4 – owner 2026-10-20: "One band per shot NOW"): a
	// picked shot. Its head, one line – the fold button (round), the review dot, the id, the format, "n done · running ·
	// cost"; it sticks while its band scrolls by; the CURRENT shot's head inverted, as the shot list's row. FOLDED (not
	// remembered): the head line alone, a click on it unfolds it; it loads nothing, the keys skip it. Its body: the cards
	// and strips where storyboard-board-layout.ts put them, the lines under them, the rows' labels, the current card's
	// actions under it. The body is filled once it comes near the view (or holds the current card). FILES dragged over
	// the band outline it; dropped, they are source cards for THIS band's shot.
	import { Icon } from '@scutoons/ui';
	import { shotSvg } from '@scutoons/content/storyboard';
	import { m } from '$lib/paraglide/messages.js';
	import { bandFacts, nodeOf, statusOf, type BandLayout, type BoardNode } from '$lib/editor/storyboard-board-layout.ts';
	import { captionOf, runningOf, stripTitle, titleOf, type WordsAt } from '$lib/editor/storyboard-board-words.ts';
	import { euro } from '$lib/editor/storyboard-money.ts';
	import { near } from '$lib/editor/storyboard-posters.ts';
	import { svgSource, type ShotEntry } from '$lib/editor/storyboard-shots.ts';
	import type { Generation } from '$lib/editor/storyboard-generations.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import VariantsLines from '$lib/components/storyboard/variants-lines.svelte';
	import VariantCard from '$lib/components/storyboard/variant-card.svelte';
	import VariantStrip from '$lib/components/storyboard/variant-strip.svelte';
	import VariantRunning from '$lib/components/storyboard/variant-running.svelte';
	import VariantActions from '$lib/components/storyboard/variant-actions.svelte';

	type Band = { shot: string; entry: ShotEntry | null; records: Generation[]; layout: BandLayout; folded: boolean };
	let { sb, band, current, cur, letters, onpick, onband, onfold, ondrop }: { sb: StoryboardPage; band: Band; current: boolean; cur: string | null; letters: Map<string, string>; onpick: (node: BoardNode, e: MouseEvent) => void; onband: () => void; onfold: () => void; ondrop: (files: File[]) => void } = $props();
	const shot = $derived(band.shot);
	const layout = $derived(band.layout);
	const rate = $derived(sb.gen.budget?.rate ?? null);
	const facts = $derived(bandFacts(band.records, rate));
	const running = $derived(band.records.filter((g) => g.status === 'queued' || g.status === 'running').length);
	const data = $derived(band.entry?.shot);
	const head = $derived([`${data?.format ?? ''} ${data?.aspect ?? ''}`.trim(), m['storyboard.board_done']({ n: facts.done }), running ? m['storyboard.board_running']({ n: running }) : '', facts.eur > 0 ? euro(facts.eur) : ''].filter(Boolean).join(' · '));
	const words = $derived<WordsAt>({ shot, generators: sb.gen.generators, rate, local: sb.localFiles, all: sb.generations });
	const curNode = $derived(cur ? (layout.tree.get(cur) ?? null) : null);
	const curAt = $derived(cur ? nodeOf(layout, cur) : null);
	const captions = $derived(layout.card.meta > 0);
	/** the render's picture: the shot's schematic as the creator draws it */
	const render = $derived.by(() => {
		if (!band.entry) return null;
		try {
			return svgSource(shotSvg(band.entry.shot, band.entry.scene).svg);
		} catch {
			return null;
		}
	});
	let seen = $state(false);

	function fold(e: MouseEvent) {
		e.stopPropagation();
		const was = band.folded;
		onfold();
		if (was) onband();
	}
	let over = $state(false);
	const carriesFiles = (e: DragEvent) => [...(e.dataTransfer?.types ?? [])].includes('Files');
	function dragover(e: DragEvent) {
		if (!carriesFiles(e)) return;
		e.preventDefault();
		if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
		over = true;
	}
	function dragleave(e: DragEvent) {
		if (!(e.currentTarget as Element).contains(e.relatedTarget as Node | null)) over = false;
	}
	function drop(e: DragEvent) {
		over = false;
		if (!carriesFiles(e)) return;
		e.preventDefault();
		ondrop([...(e.dataTransfer?.files ?? [])]);
	}
	const reviewWord = (s: string | undefined) => (s === 'ok' ? m['storyboard.review_ok']() : s === 'change' ? m['storyboard.review_change']() : m['storyboard.review_none']());
	const genOf = (node: BoardNode) => (node.record ? (band.records.find((g) => g.id === node.record!.id) ?? null) : null);
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<section class="band" class:current class:folded={band.folded} class:over data-band={shot} aria-label={shot} aria-current={current ? 'true' : undefined} onclick={onband} ondragover={dragover} ondragleave={dragleave} ondrop={drop} data-testid="band-{shot}">
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<header class="head" title={band.folded ? m['storyboard.board_band_folded']({ shot }) : m['storyboard.board_band_hint']({ shot })} onclick={() => band.folded && onfold()}>
		<span class="line">
			<button type="button" class="sc-round fold" aria-expanded={!band.folded} aria-label={band.folded ? m['storyboard.board_unfold']() : m['storyboard.board_fold']()} title={band.folded ? m['storyboard.board_unfold']() : m['storyboard.board_fold']()} onclick={fold} data-testid="band-fold"><Icon name={band.folded ? 'chevrons-down' : 'chevrons-up'} /></button>
			<span class="dot {data?.review?.status ?? ''}" title={[reviewWord(data?.review?.status), data?.review?.note].filter(Boolean).join(': ')}></span>
			<span class="id">{shot}</span>
			<span class="facts">{head}</span>
		</span>
	</header>
	{#if !band.folded}
		<div class="body" data-body style:width="{layout.width}rem" style:height="{layout.height}rem" use:near={() => (seen = true)}>
			{#if seen || current}
				<VariantsLines lines={layout.lines} width={layout.width} height={layout.height} />
				{#each layout.rows as row, i (i)}
					{#if row.label}<span class="label" style:left="{row.x}rem" style:top="{row.y}rem">{row.label === 'source' ? m['storyboard.board_row_source']() : m['storyboard.board_row_earlier']()}</span>{/if}
				{/each}
				<div class="cards" role="listbox" aria-label={m['storyboard.board_cards']({ shot })}>
					{#each layout.nodes as node (node.key)}
						{#if node.kind === 'running'}
							<!-- drawn after the listbox: not an option, the keys skip it -->
						{:else if node.kind === 'strip'}
							<VariantStrip {node} title={stripTitle(node, words)} current={node === curAt} onpick={(e) => onpick(node, e)} />
						{:else}
							{@const caption = captionOf(node, words)}
							{@const letter = letters.get(node.key) ?? ''}
							<VariantCard {sb} {node} g={genOf(node)} picture={node.source === 'render' ? render : null} {caption} {captions} title={titleOf(node, words)} label={[shot, ...caption, letter].filter(Boolean).join(', ')} status={statusOf(node.record)} {letter} current={node === curAt} onpick={(e) => onpick(node, e)} />
						{/if}
					{/each}
				</div>
				{#each layout.nodes.filter((n) => n.kind === 'running') as node (node.key)}
					{@const text = runningOf(node, words)}
					<VariantRunning {node} caption={text.caption} cost={text.cost} title={text.title} {captions} />
				{/each}
				{#if curNode && curAt}<VariantActions {sb} {shot} node={curNode} at={curAt} {layout} meta={layout.card.meta} />{/if}
			{/if}
		</div>
	{/if}
</section>

<style>
	.band {
		position: relative;
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.band.over::after {
		content: '';
		position: absolute;
		inset: 0;
		z-index: 4;
		border: var(--border-width-regular) dashed var(--ink);
		pointer-events: none;
	}
	.head {
		position: sticky;
		top: 0;
		z-index: 3;
		height: 2.5rem;
		background: var(--paper);
		border-bottom: var(--border-width-hairline) solid var(--ink);
		font-size: var(--type-caption-size);
		cursor: pointer;
	}
	.band.folded .head {
		border-bottom: 0;
	}
	.line {
		position: sticky;
		left: 0;
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		height: 100%;
		padding: 0 var(--space-1);
		white-space: nowrap;
	}
	.id {
		font-size: var(--type-button-size);
	}
	.facts {
		color: var(--text-muted);
	}
	.band.current .head {
		background: var(--ink);
		color: var(--paper);
	}
	.band.current .facts {
		color: var(--paper);
	}
	.band.current .fold {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--paper);
	}
	.dot {
		display: inline-block;
		width: 0.5rem;
		height: 0.5rem;
		border: var(--border-width-regular) solid currentColor;
		border-radius: 50%;
	}
	/* as the shot list's dot */
	.dot.ok {
		background: currentColor;
	}
	.dot.change {
		background: var(--state-warning);
	}
	.body {
		position: relative;
	}
	.cards {
		position: absolute;
		inset: 0;
	}
	.label {
		position: absolute;
		margin-top: 0.1rem;
		font-size: var(--type-caption-size);
		line-height: 1;
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--text-muted);
		white-space: nowrap;
		pointer-events: none;
	}
</style>
