<script>
	// The timeline along the bottom, under the 3D view and the panels (owner 2026-10-19: "Actually the timeline underneath
	// the other windows was also good"): the chapter's shots in reading order, each as wide as its reading time (the
	// rhythm), its frame, review dot and id; scene starts on the ruler. Click or drag = scrub – the playhead runs through
	// the chapter and the shot under it is selected (unsaved changes ask first); ← → the previous / next shot, Home / End;
	// Ctrl + wheel zooms (remembered), a double-click on the ruler fits. In the Handheld step the selected shot's take
	// with its trim handles is a second track (TakeTrack). The AppShell's bottom region: it folds to a rail, its height is
	// remembered (App).
	import { FoldButton, Icon } from '@scutoons/ui';
	import { allShots, ch, pickShot, select, studio } from '../studio.svelte.js';
	import TakeTrack from './TakeTrack.svelte';

	/** @type {{ take: boolean, onfold: () => void }} */
	let { take, onfold } = $props();
	const ZOOM_KEY = 'storyboard-studio:timeline-zoom';
	let zoom = $state((() => { try { return Math.min(16, Math.max(1, Number(localStorage.getItem(ZOOM_KEY)) || 1)); } catch { return 1; } })());
	let scroller = $state();
	let fit = $state(0);
	let rem = $state(16);
	/** the playhead, in seconds of the chapter */
	let t = $state(0);
	let scrubbed = '';

	const shots = $derived(allShots());
	const total = $derived(shots.reduce((a, s) => a + s.seconds, 0) || 1);
	/** each shot's place: px from the strip's start and its width (at least a sliver – the rhythm stays readable) */
	const lane = $derived.by(() => {
		const pps = (Math.max(0, fit - 2) / total) * zoom;
		let x = 0, sec = 0;
		return shots.map((s, i) => {
			const w = Math.max(0.5 * rem, s.seconds * pps);
			const box = { ...s, x, w, start: sec, scene: s.scene, sceneStart: i === 0 || shots[i - 1].scene !== s.scene };
			x += w;
			sec += s.seconds;
			return box;
		});
	});
	const width = $derived(lane.length ? lane[lane.length - 1].x + lane[lane.length - 1].w : 0);
	const sel = $derived(lane.find((b) => b.id === studio.sel?.id));
	const head = $derived(sel ? sel.x + (Math.min(sel.seconds, Math.max(0, t - sel.start)) / sel.seconds) * sel.w : 0);
	const clock = (s) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`;

	$effect(() => {
		void fit;
		rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	});
	// a shot picked elsewhere (the list, the stepper): the playhead to its start, the strip scrolls it into view
	$effect(() => {
		const b = sel;
		if (!b) return;
		if (scrubbed !== b.id) t = b.start;
		scrubbed = '';
		if (scroller && (b.x < scroller.scrollLeft || b.x + b.w > scroller.scrollLeft + scroller.clientWidth)) scroller.scrollLeft = Math.max(0, b.x - scroller.clientWidth / 3);
	});

	/** px on the strip → the shot there and the chapter time */
	function at(px) {
		const b = lane.find((x) => px < x.x + x.w) ?? lane[lane.length - 1];
		return b ? { b, time: b.start + Math.min(1, Math.max(0, (px - b.x) / b.w)) * b.seconds } : null;
	}
	function scrubTo(e, strip) {
		const hit = at(e.clientX - strip.getBoundingClientRect().left);
		if (!hit) return true;
		if (hit.b.id !== studio.sel?.id) {
			// unsaved changes: the drag ends and the in-page confirmation asks (the playhead follows once it is picked)
			if (studio.dirty) { pickShot(hit.b.id); return false; }
			scrubbed = hit.b.id;
			select(hit.b.id);
		}
		t = hit.time;
		return true;
	}
	function down(e) {
		if (e.button !== 0) return;
		const strip = e.currentTarget;
		strip.focus({ preventScroll: true });
		if (!scrubTo(e, strip)) return;
		strip.setPointerCapture(e.pointerId);
		const move = (ev) => { if (!scrubTo(ev, strip)) up(); };
		const up = () => { strip.removeEventListener('pointermove', move); strip.removeEventListener('pointerup', up); strip.removeEventListener('pointercancel', up); };
		strip.addEventListener('pointermove', move);
		strip.addEventListener('pointerup', up);
		strip.addEventListener('pointercancel', up);
	}
	function key(e) {
		const i = lane.findIndex((b) => b.id === studio.sel?.id);
		const to = { ArrowLeft: i - 1, ArrowRight: i + 1, Home: 0, End: lane.length - 1 }[e.key];
		if (to === undefined) return;
		e.preventDefault();
		pickShot(lane[Math.max(0, Math.min(lane.length - 1, to))]?.id);
	}
	function wheel(e) {
		if (!(e.ctrlKey || e.metaKey)) return;
		e.preventDefault();
		const before = e.clientX - scroller.getBoundingClientRect().left + scroller.scrollLeft;
		const old = zoom;
		zoom = Math.min(16, Math.max(1, zoom * (e.deltaY < 0 ? 1.25 : 0.8)));
		try { localStorage.setItem(ZOOM_KEY, String(zoom)); } catch { /* not remembered */ }
		// the point under the pointer stays where it is
		requestAnimationFrame(() => (scroller.scrollLeft = before * (zoom / old) - (e.clientX - scroller.getBoundingClientRect().left)));
	}
	function fitAll() {
		zoom = 1;
		try { localStorage.removeItem(ZOOM_KEY); } catch { /* not remembered */ }
	}
	const hidden = (e) => e.currentTarget.classList.add('missing');
</script>

<div class="timeline">
	<header>
		<h2><Icon name="film" /> Timeline</h2>
		<span class="cap">{shots.length} shots · {clock(total)}{sel ? ` · ${sel.id} at ${clock(t)}` : ''}</span>
		<span class="grow"></span>
		<span class="cap muted">drag = scrub · ← → shot · Ctrl + wheel = zoom</span>
		<FoldButton side="bottom" label="Fold the timeline" controls="sc-shell-bottom" onclick={onfold} testid="timeline-fold" />
	</header>
	<div class="tracks">
		<div class="row">
			<span class="label">Shots</span>
			<div class="scroller" bind:this={scroller} bind:clientWidth={fit} onwheel={wheel}>
				<!-- the strip is the scrub surface (a slider over the chapter's shots); the shots inside are its marks -->
				<div class="strip" style:width="{width}px" role="slider" tabindex="0" aria-label="Playhead – the selected shot" aria-valuemin="1" aria-valuemax={lane.length} aria-valuenow={(lane.indexOf(sel) ?? 0) + 1} aria-valuetext={sel ? `${sel.id} at ${clock(t)}` : 'no shot'} onpointerdown={down} onkeydown={key} data-testid="timeline-strip">
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div class="ruler" ondblclick={fitAll} title="Double-click: fit the chapter">
						{#each lane as b (b.id)}{#if b.sceneStart}<span class="scene" style:left="{b.x}px">{b.scene}</span>{/if}{/each}
					</div>
					<div class="shots">
						{#each lane as b (b.id)}
							<div class="shot" class:selected={b.id === studio.sel?.id} class:scene-start={b.sceneStart} style:left="{b.x}px" style:width="{b.w}px" title="{b.id} · {b.seconds.toFixed(1)} s{b.shot.review ? ` · ${b.shot.review.status}` : ''}{b.shot.action?.de ? ` – ${b.shot.action.de}` : ''}">
								<img src={`/out/${ch()}/frames/${b.id}.png?v=${studio.version}`} alt="" loading="lazy" draggable="false" onerror={hidden} />
								<span class="tag"><span class="dot {b.shot.review?.status ?? ''}"></span>{#if b.w > 4 * rem}{b.id.replace(/^sb-/, '')}{/if}{#if studio.data?.handheld?.[b.id] != null && b.w > 2.5 * rem}<Icon name="smartphone" size="0.9em" />{/if}</span>
							</div>
						{/each}
					</div>
					{#if sel}<span class="head" style:left="{head}px"></span>{/if}
				</div>
			</div>
		</div>
		{#if take}<TakeTrack />{/if}
	</div>
</div>

<style>
	.timeline {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		background: var(--paper);
	}
	header {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		flex: none;
		padding: var(--space-05) var(--space-05) var(--space-05) var(--space-1);
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
		font-size: var(--type-caption-size);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		min-width: 0;
		font-variant-numeric: tabular-nums;
	}
	.muted {
		color: var(--text-muted);
	}
	.grow {
		flex: 1;
	}
	.tracks {
		flex: 1 1 auto;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		padding: 0 var(--space-1) var(--space-05);
		overflow-y: auto;
	}
	.row {
		flex: 1 1 auto;
		min-height: 4.5rem;
		display: grid;
		grid-template-columns: 5rem minmax(0, 1fr);
		gap: var(--space-05);
	}
	.label {
		padding-top: 1.25rem;
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.scroller {
		min-width: 0;
		overflow-x: auto;
		overflow-y: hidden;
		border: var(--border-width-regular) solid var(--ink);
	}
	.strip {
		position: relative;
		height: 100%;
		min-width: 100%;
		cursor: ew-resize;
		touch-action: none;
		user-select: none;
	}
	.strip:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	.ruler {
		position: relative;
		height: 1.25rem;
		border-bottom: var(--border-width-regular) solid var(--ink);
		font-size: var(--type-caption-size);
		line-height: 1.25rem;
	}
	.scene {
		position: absolute;
		top: 0;
		padding-left: var(--space-025);
		border-left: var(--border-width-regular) solid var(--ink);
		white-space: nowrap;
	}
	.shots {
		position: absolute;
		top: 1.25rem;
		bottom: 0;
		left: 0;
		right: 0;
	}
	.shot {
		position: absolute;
		top: 0;
		bottom: 0;
		box-sizing: border-box;
		overflow: hidden;
		background: var(--ink);
		border-right: var(--border-width-hairline) solid var(--paper);
	}
	.shot.scene-start {
		border-left: var(--border-width-regular) solid var(--paper);
	}
	img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		object-position: top;
		display: block;
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
	.selected {
		outline: var(--border-width-focus) solid var(--state-warning);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	.selected .tag {
		background: var(--ink);
		color: var(--paper);
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
	.head {
		position: absolute;
		top: 0;
		bottom: 0;
		width: var(--border-width-focus);
		margin-left: calc(var(--border-width-focus) / -2);
		background: var(--state-warning);
		pointer-events: none;
	}
</style>
