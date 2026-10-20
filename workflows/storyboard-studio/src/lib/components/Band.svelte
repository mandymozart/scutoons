<script>
	// One band of the variants board (agents/research/VARIANTS-CANVAS.md §3.2, Q4 – owner 2026-10-20: "One band per shot
	// NOW"): a picked shot. Its head, one line – the fold button (round, 1rem from the left, as the bottom region's rail),
	// the review dot, the id, the format, "n done · cost" (euro – the provider's figures in the tooltip); it sticks while
	// its band scrolls by; the CURRENT shot's head inverted, as the shot list's row. FOLDED (DOCKING's fold to a rail,
	// lying down – not remembered): the head line alone; a click on it unfolds it; it loads nothing, the keys skip it.
	// Its body: the cards and strips where board-layout.js put them (the sources in column 0 – the render, the layout
	// variants, the takes, the dropped files –, rows of results; a rejected card a Strip; a running job a Running
	// placeholder at its row's end), the lines under them (BoardLines), the rows' labels, the current card's actions under
	// it (CardActions). The body is drawn at its height but filled only once it comes near the view (posters.js near – §5
	// "posters only, for the bands in view") or holds the current card. A click in the band makes its shot current
	// (Board). FILES dragged over the band outline it (ink, 2px dashed – the old panel's drop zone); dropped, they are
	// uploads for THIS band's shot (board-sources.svelte.js) – folded or not.
	import { Icon } from '@scutoons/ui';
	import { bandFacts, nodeOf, statusOf } from '../board-layout.js';
	import { captionOf, runningOf, stripTitle, titleOf } from '../board.svelte.js';
	import { carriesFiles } from '../board-sources.svelte.js';
	import { euro, usdTitle } from '../generate.svelte.js';
	import { near } from '../posters.js';
	import { ch, studio } from '../studio.svelte.js';
	import BoardLines from './BoardLines.svelte';
	import VariantCard from './VariantCard.svelte';
	import Strip from './Strip.svelte';
	import Running from './Running.svelte';
	import CardActions from './CardActions.svelte';

	/** @type {{ band: { shot: string, data: any, takes: Map<string, any>, records: any[], jobs: any[], layout: any, folded: boolean }, current: boolean, cur: string|null, letters: Map<string, string>, onpick: (node: any, e: MouseEvent) => void, onband: () => void, onfold: () => void, onstatus: (node: any, status: string) => void, onaction: (node: any, action: string) => void, ondrop: (files: File[]) => void }} */
	let { band, current, cur, letters, onpick, onband, onfold, onstatus, onaction, ondrop } = $props();
	const shot = $derived(band.shot);
	const layout = $derived(band.layout);
	const facts = $derived(bandFacts(band.records));
	const review = $derived(band.data?.review);
	const format = $derived(band.data ? `${band.data.format ?? ''} ${band.data.aspect ?? ''}`.trim() : '');
	const running = $derived(band.jobs?.length ?? 0);
	const head = $derived([format, `${facts.done} done`, running && `${running} running`, facts.usd > 0 && euro(facts.usd)].filter(Boolean).join(' · '));
	const headTitle = $derived(band.folded
		? `${shot} – folded: a click on the line unfolds it (the keys skip it, it loads nothing)`
		: `${shot}${current ? ' – the current shot: the panels edit it' : ' – a click in its band makes it the current shot'} · ${facts.done} result${facts.done === 1 ? '' : 's'}${facts.rejected ? ` (${facts.rejected} rejected)` : ''}${running ? `, ${running} running` : ''}${facts.usd > 0 ? ` – ${euro(facts.usd)} spent on its jobs (${usdTitle({ usd: facts.usd })})` : ''}
Drop images or videos onto the band: source cards for ${shot}`);
	/** the current card (the tree's) and the box that shows it – its card, or the strip it is folded into */
	const curNode = $derived(cur ? layout.tree.get(cur) : null);
	const curAt = $derived(cur ? nodeOf(layout, cur) : null);
	const captions = $derived(layout.card.meta > 0);
	/** near the view (or the current band): the cards are drawn, their pictures load as they come near */
	let seen = $state(false);

	/** a card's picture: the render (clean → the saved frame), a take's render, the record's file */
	function mediaOf(node) {
		const v = studio.version;
		if (node.source === 'render') return { url: `/out/${ch()}/clean/${shot}.png?v=${v}`, fallback: `/out/${ch()}/frames/${shot}.png?v=${v}`, video: false };
		// a kept layout: its saved frame (Keep as variant – POST /api/frame with the variant); none = "no picture yet"
		if (node.source === 'variant') return { url: `/out/${ch()}/variants/${shot}--${node.variant.id}.png?v=${v}`, video: false };
		if (node.source === 'take') {
			const t = band.takes.get(node.take);
			return { url: t?.url ?? null, video: true, ms: t?.ms ?? null, pass: t?.pass };
		}
		const r = node.record;
		return { url: r.url, video: r.kind === 'video', ms: r.ms ?? (r.duration ? r.duration * 1000 : null), missing: r.missing };
	}
	const cardLabel = (node, caption, letter) => [shot, ...caption, statusOf(node.record), letter && `letter ${letter}`].filter(Boolean).join(', ');
	/** the fold button: folds (the current shot stays) – on a folded band it unfolds and makes the shot current */
	function fold(e) {
		e.stopPropagation();
		const was = band.folded;
		onfold();
		if (was) onband();
	}

	/** files dragged over the band: its outline; dropped: uploads for its shot (a card's own drag, a link: not ours) */
	let over = $state(false);
	function dragover(e) {
		if (!carriesFiles(e)) return;
		e.preventDefault();
		e.dataTransfer.dropEffect = 'copy';
		over = true;
	}
	function dragleave(e) {
		if (!e.currentTarget.contains(e.relatedTarget)) over = false;
	}
	function drop(e) {
		over = false;
		if (!carriesFiles(e)) return;
		e.preventDefault();
		ondrop([...e.dataTransfer.files]);
	}
</script>

<!-- the band's click picks its shot; the keys live on the board (↑ ↓ cross the bands) -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<section class="band" class:current class:folded={band.folded} class:over data-band={shot} aria-label="{shot}{current ? ' – the current shot' : ''}{band.folded ? ' – folded' : ''}" aria-current={current ? 'true' : undefined} onclick={onband} ondragover={dragover} ondragleave={dragleave} ondrop={drop}>
	<!-- folded: a click on the line unfolds it (then the section's click makes its shot current); the button is the keys' way -->
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<header class="head" title={headTitle} onclick={() => band.folded && onfold()}>
		<span class="line">
			<button type="button" class="sc-round fold" aria-expanded={!band.folded} aria-label={band.folded ? `Unfold ${shot}'s band` : `Fold ${shot}'s band to this line`} title={band.folded ? 'Unfold the band' : 'Fold the band to this line (it loads nothing then; the keys skip it)'} onclick={fold} data-testid="band-fold"><Icon name={band.folded ? 'chevrons-down' : 'chevrons-up'} /></button>
			<span class="dot {review?.status ?? ''}" title={review ? `${review.status} ${review.date ?? ''}: ${review.note ?? ''}` : 'no review yet'}></span>
			<span class="id">{shot}</span>
			<span class="facts">{head}</span>
		</span>
	</header>
	{#if !band.folded}
		<div class="body" data-body style:width="{layout.width}rem" style:height="{layout.height}rem" use:near={() => (seen = true)}>
			{#if seen || current}
				<BoardLines lines={layout.lines} width={layout.width} height={layout.height} />
				{#each layout.rows as row, i (i)}
					{#if row.label}<span class="label" style:left="{row.x}rem" style:top="{row.y}rem">{row.label}</span>{/if}
				{/each}
				<!-- the cards: a listbox (a composite – screen readers hand it the arrow keys), its selected option the current
				     card (selection follows focus); Compare's letters A–D are in the cards' names; the current card's actions
				     outside it -->
				<div class="cards" role="listbox" aria-label="{shot}'s cards – the arrows move, Enter chosen, R review, X reject">
					{#each layout.nodes as node (node.key)}
						{#if node.kind === 'running'}
							<!-- drawn after the listbox: not an option, the keys skip it -->
						{:else if node.kind === 'strip'}
							<Strip {node} title={stripTitle(node, shot, layout.tree)} current={node === curAt} onpick={(e) => onpick(node, e)} />
						{:else}
							{@const media = mediaOf(node)}
							{@const caption = captionOf(node, media)}
							<VariantCard {node} {media} {caption} {captions} title={titleOf(node, shot, media)} label={cardLabel(node, caption, letters.get(node.key))} status={statusOf(node.record)} letter={letters.get(node.key) ?? ''} current={node === curAt} onpick={(e) => onpick(node, e)} />
						{/if}
					{/each}
				</div>
				{#each layout.nodes.filter((n) => n.kind === 'running') as node (node.key)}
					{@const text = runningOf(node, shot)}
					<Running {node} caption={text.caption} cost={text.cost} title={text.title} {captions} />
				{/each}
				{#if curNode && curAt}<CardActions node={curNode} at={curAt} meta={layout.card.meta} onstatus={(s) => onstatus(curNode, s)} onaction={(a) => onaction(curNode, a)} />{/if}
			{/if}
		</div>
	{/if}
</section>

<style>
	.band {
		position: relative;
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	/* files dragged over it: the drop target – ink, 2px dashed (the old panel's drop zone), over its head and its cards */
	.band.over::after {
		content: '';
		position: absolute;
		inset: 0;
		z-index: 4;
		border: var(--border-width-regular) dashed var(--ink);
		pointer-events: none;
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
	/* the current shot (the one the panels edit): inverted, as the shot list's row – its fold button too */
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
	.band.current .fold:hover {
		outline-color: var(--paper);
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
