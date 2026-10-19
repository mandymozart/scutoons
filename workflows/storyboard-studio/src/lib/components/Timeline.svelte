<script>
	// The studio's timeline along the bottom, under the 3D view and the panels (owner 2026-10-19: "Actually the timeline
	// underneath the other windows was also good"): the shared @scutoons/ui Timeline (creator-ready – research/TIMELINE.md)
	// with the studio's lanes (timeline.svelte.js): the Shots lane – each shot as wide as its reading time, its frame,
	// review dot and handheld mark, the scenes on the ruler – and in the Handheld step the selected shot's take at the
	// shot's start with its trim. The header: the measures, Play for the take, the fold. AppShell bottom region (App).
	import { FoldButton, Icon, IconButton, Timeline } from '@scutoons/ui';
	import { formatTime } from '@scutoons/ui/timeline';
	import { ch, studio } from '../studio.svelte.js';
	import { hh } from '../handheld/handheld.svelte.js';
	import { followPlayhead, keepZoom, loadZoom, seek, select, timelineOf, tl, togglePlay, trim } from '../timeline.svelte.js';

	/** @type {{ step: string, onfold: () => void }} */
	let { step, onfold } = $props();
	const view = $derived(timelineOf(step === 'handheld'));
	followPlayhead(() => view);
	// the zoom per chapter and step: read when either changes, kept when it changes; in Handheld the take's reveal sets it
	// (zoomed to the take to trim it) – reading a remembered zoom there would undo that
	$effect(() => { void studio.chapter; if (step !== 'handheld') loadZoom(step); });
	$effect(() => { keepZoom(tl.zoom, step); });
	const selected = $derived(view.sel ? [view.sel.id] : []);
	// a frame that is still missing stays black (the frames are rendered by "All frames" / Save + frame)
	const missing = (e) => e.currentTarget.classList.add('missing');
</script>

{#snippet item(it, lane)}
	{#if lane.id === 'shots'}
		<img src={`/out/${ch()}/frames/${it.id}.png?v=${studio.version}`} alt="" loading="lazy" draggable="false" onerror={missing} onload={(e) => e.currentTarget.classList.remove('missing')} />
		<span class="tag"><span class="dot {it.state}"></span><span class="id">{it.id.replace(/^sb-/, '')}</span>{#if studio.data?.handheld?.[it.id] != null}<Icon name="smartphone" size="0.9em" />{/if}</span>
	{:else}
		<span class="tag"><span class="id">{it.label}</span></span>
	{/if}
{/snippet}

<div class="studio-timeline">
	<header>
		<h2><Icon name="film" /> Timeline</h2>
		<span class="cap" title="Reading time: the shots' measured seconds (scripts/storyboard RHYTHM)">{view.lanes[0].items.length} shots · ≈ {formatTime(view.span)} reading{view.sel ? ` · ${view.sel.id} at ${formatTime(tl.playhead)}` : ''}</span>
		<span class="grow"></span>
		{#if view.take}
			<span class="cap">Take {view.take.items[0].id} · kept {formatTime(view.take.items[0].trim.out - view.take.items[0].trim.in, 's')}</span>
			<IconButton label={hh.playing ? 'Pause' : 'Play the kept part'} class="sc-round" onclick={togglePlay}><Icon name={hh.playing ? 'pause' : 'play'} /></IconButton>
		{/if}
		<FoldButton side="bottom" label="Fold the timeline" controls="sc-shell-bottom" onclick={onfold} testid="timeline-fold" />
	</header>
	<Timeline
		label="Timeline"
		lanes={view.lanes}
		span={view.span}
		markers={view.markers}
		playhead={tl.playhead}
		{selected}
		bind:zoom={tl.zoom}
		reveal={view.reveal}
		revealFill={view.take ? 0.5 : 0}
		follow={hh.playing}
		{item}
		onseek={(t, final) => seek(view, t, final)}
		onselect={select}
		ontrim={(lane, id, kept, final) => trim(view, lane, id, kept, final)}
	/>
</div>

<style>
	.studio-timeline {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		background: var(--paper);
	}
	.studio-timeline > :global(:last-child) {
		flex: 1 1 auto;
		min-height: 0;
	}
	header {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		flex: none;
		padding: var(--space-05) var(--space-05) var(--space-025) var(--space-1);
	}
	h2 {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		margin: 0;
		font-size: var(--type-label-size);
		line-height: var(--type-label-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		white-space: nowrap;
	}
	.cap {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--type-caption-size);
		font-variant-numeric: tabular-nums;
	}
	.grow {
		flex: 1;
	}
	/* an item's inside (the shared Timeline positions and frames it) */
	img {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: cover;
		object-position: top;
		pointer-events: none;
	}
	img:global(.missing) {
		visibility: hidden;
	}
	.tag {
		position: absolute;
		left: 0;
		top: 0;
		display: flex;
		align-items: center;
		gap: 0.125rem;
		max-width: 100%;
		padding: 0 0.125rem;
		background: var(--paper);
		color: var(--ink);
		font-size: var(--type-caption-size);
		line-height: 1.2;
		white-space: nowrap;
		overflow: hidden;
	}
	.dot {
		flex: none;
		width: 0.5rem;
		height: 0.5rem;
		box-sizing: border-box;
		border: var(--border-width-regular) solid currentColor;
		border-radius: 50%;
	}
	.dot.ok {
		background: var(--state-success);
	}
	.dot.change {
		background: var(--state-warning);
	}
	/* narrow shots (the whole chapter fitted): the id goes first, then the tag – the frame and the rhythm stay */
	@container (max-width: 4rem) {
		.id,
		.tag :global(svg) {
			display: none;
		}
	}
	@container (max-width: 1.25rem) {
		.tag {
			display: none;
		}
	}
</style>
