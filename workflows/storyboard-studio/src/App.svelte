<script>
	// The storyboard studio (owner 2026-10-09: "use our scutoons ui and design dogmas"; 2026-10-19: research/STUDIO-LAYOUT.md
	// concept C, stage 1 = step tabs on the dock). One AppShell like the tools, the same in every step ("it makes sense for
	// it to always look the same"): the shots as the sidebar (the toon's order – a click picks one, Ctrl / Shift + click
	// more), the main view as the page, the step's panels in the right dock (each with the creator's head, folds to its
	// rail, drags wider), the timeline along the bottom under the view and the panels (owner: "the timeline underneath the
	// other windows was also good") – the picked shots' clips in the step's view mode –, the ActionBar. The main view
	// follows the clip under the playhead: the 3D view (a still, the current shot's take) or ClipView over it (a video, or
	// another shot's frame) – the 3D view stays mounted underneath (WebGL). A step only changes which panels are open, the
	// Shot panel's tab, the view mode, the timeline's fold and the panels' widths (steps.svelte.js). While a take is
	// trimmed the main view shows the in / out pair (TrimPair). Keys outside text fields: 1–6 the step, T the shot list,
	// N the step's panels, I / O the trim at the playhead (Take mode); Ctrl + Space, Ctrl + \, Ctrl + Shift + \ as in the
	// creator. agents/DOCKING.md, DESIGN.md.
	import { onMount, untrack } from 'svelte';
	import { AppShell, DialogHost, installTooltips, panelHeight, panelWidth, Segmented, shellDock, ToastStack, useDock } from '@scutoons/ui';
	import { fail, loadChapters, studio, toaster } from './lib/studio.svelte.js';
	import { endPair, stop, timelineOf, tl, trimAtPlayhead, useTimeline } from './lib/timeline.svelte.js';
	import { clipAt, isVideo } from './lib/timeline-lanes.js';
	import { here, isOpen, PANELS, setOpen, setStep, setTab, setTimeline, SHOT_TABS, STEPS, steps } from './lib/steps.svelte.js';
	import { followSelection } from './lib/handheld/handheld.svelte.js';
	import TopBar from './lib/components/TopBar.svelte';
	import ShotList from './lib/components/ShotList.svelte';
	import Viewport from './lib/components/Viewport.svelte';
	import Timeline from './lib/components/Timeline.svelte';
	import ClipView from './lib/components/ClipView.svelte';
	import TrimPair from './lib/components/TrimPair.svelte';
	import StudioActions from './lib/components/StudioActions.svelte';
	import StudioPanel from './lib/components/StudioPanel.svelte';
	import ShotPanel from './lib/components/ShotPanel.svelte';
	import ScriptPanel from './lib/components/ScriptPanel.svelte';
	import VariantsPanel from './lib/components/VariantsPanel.svelte';
	import HandheldPanel from './lib/components/HandheldPanel.svelte';
	import GeneratePanel from './lib/components/GeneratePanel.svelte';
	import OutputPanel from './lib/components/OutputPanel.svelte';

	// every step's own panel widths (remembered per step and kind)
	const sizes = Object.fromEntries(STEPS.map((s) => [s.id, Object.fromEntries(PANELS.map((p) => [p.kind, panelWidth(`storyboard-studio:${s.id}:${p.kind}`, p.rem, undefined, { foldable: true })]))]));
	// read when the dock asks (the snippets are declared below)
	const body = (kind) => ({ shot: shotBody, script: scriptBody, variants: variantsBody, handheld: handheldBody, generate: generateBody, output: outputBody })[kind];
	const panel = (kind) => PANELS.find((p) => p.kind === kind);
	// the Shot panel names the shot (• = unsaved); the others are about the same shot – their word is enough
	const titleOf = (p) => (p.kind === 'shot' && studio.sel ? `Shot · ${studio.sel.id}${studio.dirty ? ' •' : ''}` : p.title);
	// the squeeze's floor (DOCKING "Several panels"): the compact inspector's rows need about 18rem – narrower, the
	// least-recently-used panel folds to its rail instead of the contents colliding
	const MIN = 18;
	PANELS.forEach((p, order) =>
		useDock(() => (isOpen(p.kind) && (studio.sel || p.kind === 'output') ? { kind: `storyboard-${p.kind}`, order, min: MIN, size: sizes[steps.current][p.kind], title: titleOf(p), icon: p.icon, close: () => setOpen(p.kind, false), panel: body(p.kind) } : null))
	);

	// the timeline: one height for all steps, its fold per step (both ways – the fold button / a drag write the step's)
	const timeline = panelHeight('storyboard-studio:timeline', 12);
	$effect(() => {
		const folded = !here().timeline;
		untrack(() => timeline.fold(folded));
	});
	$effect(() => {
		const open = !timeline.folded;
		// folded, its Pause is out of sight: Play ends (and a trim's pair – a gesture its lane lost)
		untrack(() => {
			setTimeline(open);
			if (!open) { stop(); endPair(); }
		});
	});
	// a maximized region rails the timeline without folding it – its Pause is out of sight too: Play ends
	$effect(() => {
		if (shellDock.maximized) untrack(() => { stop(); endPair(); });
	});

	// the timeline: the picked shots' clips in the step's view mode, the clip under the playhead (the main view follows it)
	const view = $derived(timelineOf(here().view));
	const cur = $derived(clipAt(view.items, tl.playhead));
	// ClipView over the 3D view: a video clip – or a still / take of a shot that isn't the current one (its frame)
	const clip = $derived(cur && (isVideo(cur.item.clip.source) || cur.item.id !== studio.sel?.id) ? cur : null);

	followSelection();
	useTimeline(() => view, () => cur);
	onMount(() => {
		installTooltips();
		loadChapters().catch(fail);
	});

	// 1–6, T, N, I / O – only outside text fields and dialogs, without modifiers (Blender-like; owner 2026-10-19)
	const typing = (el) => !!el?.closest?.('input, textarea, select, [contenteditable]:not([contenteditable="false"])');
	function keys(e) {
		if (e.ctrlKey || e.metaKey || e.altKey || e.repeat || e.defaultPrevented || typing(document.activeElement) || document.querySelector('dialog[open]')) return;
		const n = Number(e.key);
		if (n >= 1 && n <= STEPS.length) setStep(STEPS[n - 1].id);
		else if (e.key === 't' || e.key === 'T') window.dispatchEvent(new KeyboardEvent('keydown', { key: '\\', code: 'Backslash', ctrlKey: true }));
		else if (e.key === 'n' || e.key === 'N') foldPanels();
		// I / O: the take under the playhead trimmed there – only where the timeline shows (research/TIMELINE.md §4)
		else if (/^[io]$/i.test(e.key)) trimAtPlayhead(!timeline.folded && !shellDock.maximized ? view : null, e.key.toLowerCase() === 'i' ? 'in' : 'out');
		else return;
		e.preventDefault();
	}
	/** N: the step's panels fold to their rails – or come back (none open: the step's own set opens) */
	function foldPanels() {
		const open = PANELS.filter((p) => isOpen(p.kind));
		if (!open.length) return STEPS.find((s) => s.id === steps.current).panels.forEach((k) => setOpen(k, true));
		const fold = open.some((p) => !sizes[steps.current][p.kind].folded);
		open.forEach((p) => sizes[steps.current][p.kind].fold(fold));
	}
</script>

<svelte:window onkeydown={keys} />

{#snippet shotTabs()}
	<Segmented label="Shot panel" stretch radio options={SHOT_TABS} value={here().tab} onchange={setTab} />
{/snippet}
{#snippet shotBody()}<StudioPanel title={titleOf(panel('shot'))} icon="sliders" onclose={() => setOpen('shot', false)} tabs={shotTabs} testid="panel-shot"><ShotPanel tab={here().tab} /></StudioPanel>{/snippet}
{#snippet scriptBody()}<StudioPanel title={titleOf(panel('script'))} icon="file-text" onclose={() => setOpen('script', false)}><ScriptPanel /></StudioPanel>{/snippet}
{#snippet handheldBody()}<StudioPanel title={titleOf(panel('handheld'))} icon="smartphone" onclose={() => setOpen('handheld', false)}><HandheldPanel /></StudioPanel>{/snippet}
{#snippet generateBody()}<StudioPanel title={titleOf(panel('generate'))} icon="zap" onclose={() => setOpen('generate', false)}><GeneratePanel /></StudioPanel>{/snippet}
{#snippet variantsBody()}<StudioPanel title={titleOf(panel('variants'))} icon="grid" onclose={() => setOpen('variants', false)}><VariantsPanel /></StudioPanel>{/snippet}
{#snippet outputBody()}<StudioPanel title="Output" icon="download" onclose={() => setOpen('output', false)}><OutputPanel /></StudioPanel>{/snippet}

{#snippet bottom()}
	<Timeline {view} onfold={() => timeline.fold(true)} />
{/snippet}

<div class="studio sc-tool">
	<AppShell mode="fill" minContent={24} resizeKey="storyboard-studio:shots" foldable railLabel="Shots" sidebarLabel="Shots" sidebarTag="aside" dockLabel="Shot panels" pageLabel="3D view" {bottom} bottomSize={timeline} bottomLabel="Timeline" bottomIcon="film" bottomResizeLabel="Resize the timeline" bottomUnfoldLabel="Show the timeline">
		{#snippet topbar()}
			<TopBar />
		{/snippet}
		{#snippet sidebar()}
			<ShotList />
		{/snippet}
		<div class="main">
			<div class="three" inert={!!clip || !!tl.pair}><Viewport /></div>
			{#if tl.pair}<TrimPair pair={tl.pair} step={view.lanes[0]?.step} />{:else if clip}<ClipView {clip} playing={tl.playing} />{/if}
		</div>
		<StudioActions />
	</AppShell>
	<ToastStack {toaster} />
	<DialogHost />
</div>

<style>
	/* The studio's text size (owner 2026-10-10: "can we reduce the default font size to 60 % … better real estate for
	   this complex editor"): 60 % of the tools' fluid root (base.css: 16px, 24px from 768px) – everything is rem, so the
	   whole editor scales with it. Percent of the browser's default (16px): 60 % = 9.6px, 90 % = 14.4px. */
	:global(html:root) {
		font-size: 60%;
	}
	@media (min-width: 768px) {
		:global(html:root) {
			font-size: 90%;
		}
	}
	/* the main view: the 3D view, ClipView over it while a clip shows */
	.main {
		position: relative;
		height: 100%;
		min-height: 0;
	}
	.three {
		height: 100%;
	}
	.studio {
		height: 100dvh;
		/* the smaller root: captions and micro labels a step up so they stay readable (≈ 11.7px / 10.8px at 14.4px) */
		--type-caption-size: 0.8125rem;
		--type-nav-micro-size: 0.75rem;
		/* the bars as tall as a control plus a little air (the tools' 48px floor is for the 16px root) */
		--tool-bar-height: calc(var(--control-height) + var(--space-05));
		--action-bar-height: var(--tool-bar-height);
		/* the shots: wide enough for an id and its format on one line beside the frame */
		--tool-sidebar-width: 15rem;
	}
</style>
