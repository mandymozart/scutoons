<script>
	// One band of the variants board (agents/research/VARIANTS-CANVAS.md §3.2, Q4 – owner 2026-10-20: "One band per shot
	// NOW"): a picked shot. Its head, one line – the review dot, the id, the format, "n done · cost" (euro – the
	// provider's figures in the tooltip); it sticks while its band scrolls by; the CURRENT shot's head inverted, as the shot
	// list's row. Its body: the cards where board-layout.js put them (the sources in column 0, rows of results), the
	// lines under them (BoardLines), the rows' labels, the current card's actions under it (CardActions). A click in the
	// band makes its shot current (Board). Folding a band is stage 2b.
	import { bandFacts, statusOf } from '../board-layout.js';
	import { captionOf, titleOf } from '../board.svelte.js';
	import { euro, usdTitle } from '../generate.svelte.js';
	import { ch, studio } from '../studio.svelte.js';
	import BoardLines from './BoardLines.svelte';
	import VariantCard from './VariantCard.svelte';
	import CardActions from './CardActions.svelte';

	/** @type {{ band: { shot: string, data: any, takes: Map<string, any>, records: any[], layout: any }, current: boolean, cur: string|null, letters: Map<string, string>, onpick: (node: any, e: MouseEvent) => void, onband: () => void, onstatus: (node: any, status: string) => void }} */
	let { band, current, cur, letters, onpick, onband, onstatus } = $props();
	const shot = $derived(band.shot);
	const layout = $derived(band.layout);
	const facts = $derived(bandFacts(band.records));
	const review = $derived(band.data?.review);
	const format = $derived(band.data ? `${band.data.format ?? ''} ${band.data.aspect ?? ''}`.trim() : '');
	const head = $derived([format, `${facts.done} done`, facts.usd > 0 && euro(facts.usd)].filter(Boolean).join(' · '));
	const headTitle = $derived(`${shot}${current ? ' – the current shot: the panels edit it' : ' – a click in its band makes it the current shot'} · ${facts.done} result${facts.done === 1 ? '' : 's'}${facts.rejected ? ` (${facts.rejected} rejected)` : ''}${facts.usd > 0 ? ` – ${euro(facts.usd)} spent on its jobs (${usdTitle({ usd: facts.usd })})` : ''}`);
	const curNode = $derived(cur ? layout.nodes.find((n) => n.key === cur) : null);

	/** a card's picture: the render (clean → the saved frame), a take's render, the record's file */
	function mediaOf(node) {
		const v = studio.version;
		if (node.source === 'render') return { url: `/out/${ch()}/clean/${shot}.png?v=${v}`, fallback: `/out/${ch()}/frames/${shot}.png?v=${v}`, video: false };
		if (node.source === 'take') {
			const t = band.takes.get(node.take);
			return { url: t?.url ?? null, video: true, ms: t?.ms ?? null, pass: t?.pass };
		}
		const r = node.record;
		return { url: r.url, video: r.kind === 'video', ms: r.ms ?? (r.duration ? r.duration * 1000 : null), missing: r.missing };
	}
	const cardLabel = (node, caption, letter) => [shot, ...caption, statusOf(node.record), letter && `letter ${letter}`].filter(Boolean).join(', ');
</script>

<!-- the band's click picks its shot; the keys live on the board (↑ ↓ cross the bands) -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<section class="band" class:current data-band={shot} aria-label="{shot}{current ? ' – the current shot' : ''}" aria-current={current ? 'true' : undefined} onclick={onband}>
	<header class="head" title={headTitle}>
		<span class="line">
			<span class="dot {review?.status ?? ''}" title={review ? `${review.status} ${review.date ?? ''}: ${review.note ?? ''}` : 'no review yet'}></span>
			<span class="id">{shot}</span>
			<span class="facts">{head}</span>
		</span>
	</header>
	<div class="body" style:width="{layout.width}rem" style:height="{layout.height}rem">
		<BoardLines lines={layout.lines} width={layout.width} height={layout.height} />
		{#each layout.rows as row, i (i)}
			{#if row.label}<span class="label" style:left="{row.x}rem" style:top="{row.y}rem">{row.label}</span>{/if}
		{/each}
		<!-- the cards: a listbox (a composite – screen readers hand it the arrow keys), its selected option the current card
		     (selection follows focus); Compare's letters A–D are in the cards' names; the current card's actions outside it -->
		<div class="cards" role="listbox" aria-label="{shot}'s cards – the arrows move, Enter chosen, R review, X reject">
			{#each layout.nodes as node (node.key)}
				{@const media = mediaOf(node)}
				{@const caption = captionOf(node, media)}
				<VariantCard {node} {media} {caption} title={titleOf(node, shot, media)} label={cardLabel(node, caption, letters.get(node.key))} status={statusOf(node.record)} letter={letters.get(node.key) ?? ''} current={node.key === cur} onpick={(e) => onpick(node, e)} />
			{/each}
		</div>
		{#if curNode}<CardActions node={curNode} onstatus={(s) => onstatus(curNode, s)} />{/if}
	</div>
</section>

<style>
	.band {
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	/* the head: one line that sticks to the board's top while its band scrolls by; its words stay at the left edge when
	   the board scrolls sideways */
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
	/* the current shot (the one the panels edit): inverted, as the shot list's row */
	.band.current .head {
		background: var(--ink);
		color: var(--paper);
	}
	.band.current .facts {
		color: var(--paper);
	}
	.dot {
		display: inline-block;
		width: 0.5rem;
		height: 0.5rem;
		border: var(--border-width-regular) solid currentColor;
		border-radius: 50%;
	}
	/* the badge tokens, as the mockup's (there is no --state-success token) */
	.dot.ok {
		background: var(--badge-published);
	}
	.dot.change {
		background: var(--badge-warning);
	}
	.body {
		position: relative;
	}
	.cards {
		position: absolute;
		inset: 0;
	}
	/* a row's label above its first card (Source, Earlier – no record) */
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
