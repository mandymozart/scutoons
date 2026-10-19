<script>
	// The studio's timeline along the bottom, under the main view and the panels (owner 2026-10-19: "the timeline
	// underneath the other windows was also good"; then "the other timeline underneath the clip only shows those shots
	// that are selected, not all"): the shared @scutoons/ui Timeline (research/TIMELINE.md) with ONE lane – the picked
	// shots' clips back to back, each as long as its clip in the view mode (timeline.svelte.js, timeline-lanes.js); a clip
	// shows the shot's frame and a tag with its source (still · take · rendered · generated). The header: the view mode
	// (a square Segmented, the step's main view – "like in blender … shot/take/rendered/generated"), the clips' count and
	// length, "Back to the shot camera" while a take's moment holds the 3D view, Play / Pause for the sequence, the fold.
	// A hover shows the clip's first frame next to the pointer (the Timeline's `preview`). AppShell bottom region (App).
	import { FoldButton, Icon, IconButton, Segmented, Timeline } from '@scutoons/ui';
	import { formatTime } from '@scutoons/ui/timeline';
	import { ch, studio } from '../studio.svelte.js';
	import { here, setView, VIEW_MODES } from '../steps.svelte.js';
	import { hh } from '../handheld/handheld.svelte.js';
	import { backToShot, seek, select, tl, togglePlay, trim } from '../timeline.svelte.js';
	import { isVideo } from '../timeline-lanes.js';

	/** @type {{ view: { mode: string, lanes: any[], items: any[], span: number, sel: any }, onfold: () => void }} */
	let { view, onfold } = $props();
	const SOURCES = {
		still: { icon: 'image', word: 'still' },
		take: { icon: 'smartphone', word: 'take' },
		rendered: { icon: 'film', word: 'rendered' },
		generated: { icon: 'zap', word: 'generated' }
	};
	const selected = $derived(view.sel ? [view.sel.id] : []);
	const reveal = $derived(view.sel ? [view.sel.start, view.sel.start + view.sel.length] : undefined);
	const frame = (id) => `/out/${ch()}/frames/${id}.png?v=${studio.version}`;
	const short = (id) => id.replace(/^sb-/, '');
	// a frame that is still missing stays black (the frames are rendered by "All frames" / Save + frame)
	const missing = (e) => e.currentTarget.classList.add('missing');
</script>

{#snippet item(it)}
	{@const s = SOURCES[it.state] ?? SOURCES.still}
	<img src={frame(it.id)} alt="" loading="lazy" draggable="false" onerror={missing} onload={(e) => e.currentTarget.classList.remove('missing')} />
	<span class="tag" data-source={it.state}><Icon name={s.icon} size="0.9em" /><span class="id">{short(it.id)}</span><span class="word">{s.word}</span></span>
{/snippet}

<!-- next to the pointer: the clip's first frame (a video's frame 0; a take / a still: the shot's frame), the shot, the time -->
{#snippet preview(it, lane, t)}
	<span class="peek">
		{#if isVideo(it.clip.source)}
			<video src="{it.clip.url}#t=0.001" preload="metadata" muted playsinline></video>
		{:else}
			<img src={frame(it.id)} alt="" onerror={missing} />
		{/if}
		<span class="line">{it.id} · {SOURCES[it.state]?.word ?? ''}</span>
		<span class="line">{formatTime(t - it.start, 's', 1)} of {formatTime(it.length, 's', 1)}</span>
	</span>
{/snippet}

<div class="studio-timeline">
	<header>
		<h2><Icon name="film" /> Timeline</h2>
		<Segmented label="View mode – which clip and what the main view shows" radio options={VIEW_MODES} value={here().view} onchange={setView} />
		<span class="cap">{view.items.length} clip{view.items.length === 1 ? '' : 's'} · {formatTime(view.span)}</span>
		<span class="grow"></span>
		{#if hh.held && !hh.live}<IconButton label="Back to the shot camera" class="sc-round" onclick={backToShot}><Icon name="video" /></IconButton>{/if}
		<IconButton label={tl.playing ? 'Pause' : 'Play the clips'} class="sc-round" disabled={!view.items.length} onclick={togglePlay}><Icon name={tl.playing ? 'pause' : 'play'} /></IconButton>
		<FoldButton side="bottom" label="Fold the timeline" controls="sc-shell-bottom" onclick={onfold} testid="timeline-fold" />
	</header>
	{#if view.items.length}
		<Timeline
			label="Timeline"
			lanes={view.lanes}
			span={view.span}
			playhead={tl.playhead}
			{selected}
			bind:zoom={tl.zoom}
			{reveal}
			follow={tl.playing}
			{item}
			{preview}
			onseek={(t, final) => seek(view, t, final)}
			onselect={(lane, id) => select(view, lane, id)}
			ontrim={view.mode === 'take' ? (lane, id, kept, final) => trim(view, lane, id, kept, final) : undefined}
		/>
	{:else}
		<p class="sc-empty">Pick a shot on the left – Ctrl + click adds more to the timeline.</p>
	{/if}
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
	.sc-empty {
		margin: 0;
		padding: var(--space-05) var(--space-1);
	}
	/* a clip's inside (the shared Timeline positions and frames it) */
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
		gap: 0.25rem;
		max-width: 100%;
		padding: 0 0.25rem;
		background: var(--paper);
		color: var(--ink);
		font-size: var(--type-caption-size);
		line-height: 1.2;
		white-space: nowrap;
		overflow: hidden;
	}
	/* the source as an icon + its word (never a colour) */
	.word {
		color: var(--text-muted);
	}
	/* narrow clips: the word goes first, then the id, then the tag – the frame stays */
	@container (max-width: 8rem) {
		.word {
			display: none;
		}
	}
	@container (max-width: 3rem) {
		.id {
			display: none;
		}
	}
	@container (max-width: 1.25rem) {
		.tag {
			display: none;
		}
	}
	/* the hover preview's inside (the Timeline's box: paper, an ink frame, in the top layer) */
	.peek {
		display: grid;
		gap: 0.125rem;
		justify-items: start;
	}
	.peek video,
	.peek img {
		position: static;
		display: block;
		width: auto;
		height: 10rem;
		max-width: 16rem;
		object-fit: contain;
		background: var(--ink);
	}
	.line {
		white-space: nowrap;
	}
</style>
