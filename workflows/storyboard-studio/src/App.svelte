<script>
	// The storyboard studio (owner 2026-10-09: "use our scutoons ui and design dogmas"; 2026-10-19: research/STUDIO-LAYOUT.md
	// concept C, stage 1 = step tabs on the dock). One AppShell like the tools, the same in every step ("it makes sense for
	// it to always look the same"): the shots as the sidebar, the 3D view as the page, the step's panels in the right dock
	// (each with the creator's head, folds to its rail, drags wider), the timeline along the bottom under the view and the
	// panels (owner: "the timeline underneath the other windows was also good"), the ActionBar. A step only changes which
	// panels are open, the Shot panel's tab, the timeline's fold and the panels' widths (steps.svelte.js). Keys outside text
	// fields: 1–6 the step, T the shot list, N the step's panels; Ctrl + Space, Ctrl + \, Ctrl + Shift + \ as in the creator.
	// agents/DOCKING.md, DESIGN.md.
	import { onMount, untrack } from 'svelte';
	import { AppShell, DialogHost, installTooltips, panelHeight, panelWidth, Segmented, ToastStack, useDock } from '@scutoons/ui';
	import { fail, loadChapters, studio, toaster } from './lib/studio.svelte.js';
	import { here, isOpen, PANELS, setOpen, setStep, setTab, setTimeline, SHOT_TABS, STEPS, steps } from './lib/steps.svelte.js';
	import { followSelection } from './lib/handheld/handheld.svelte.js';
	import TopBar from './lib/components/TopBar.svelte';
	import ShotList from './lib/components/ShotList.svelte';
	import Viewport from './lib/components/Viewport.svelte';
	import Timeline from './lib/components/Timeline.svelte';
	import StudioActions from './lib/components/StudioActions.svelte';
	import StudioPanel from './lib/components/StudioPanel.svelte';
	import ShotPanel from './lib/components/ShotPanel.svelte';
	import ScriptPanel from './lib/components/ScriptPanel.svelte';
	import VariantsPanel from './lib/components/VariantsPanel.svelte';
	import PromptPanel from './lib/components/PromptPanel.svelte';
	import HandheldPanel from './lib/components/HandheldPanel.svelte';
	import GeneratePanel from './lib/components/GeneratePanel.svelte';
	import OutputPanel from './lib/components/OutputPanel.svelte';

	// every step's own panel widths (remembered per step and kind)
	const sizes = Object.fromEntries(STEPS.map((s) => [s.id, Object.fromEntries(PANELS.map((p) => [p.kind, panelWidth(`storyboard-studio:${s.id}:${p.kind}`, p.rem, undefined, { foldable: true })]))]));
	// read when the dock asks (the snippets are declared below)
	const body = (kind) => ({ shot: shotBody, script: scriptBody, variants: variantsBody, prompt: promptBody, handheld: handheldBody, generate: generateBody, output: outputBody })[kind];
	// the Shot panel names the shot (• = unsaved); the others are about the same shot – their word is enough
	const titleOf = (p) => (p.kind === 'shot' && studio.sel ? `Shot · ${studio.sel.id}${studio.dirty ? ' •' : ''}` : p.title);
	PANELS.forEach((p, order) =>
		useDock(() => (isOpen(p.kind) && (studio.sel || p.kind === 'output') ? { kind: `storyboard-${p.kind}`, order, size: sizes[steps.current][p.kind], title: titleOf(p), icon: p.icon, close: () => setOpen(p.kind, false), panel: body(p.kind) } : null))
	);

	// the timeline: one height for all steps, its fold per step (both ways – the fold button / a drag write the step's)
	const timeline = panelHeight('storyboard-studio:timeline', 12);
	$effect(() => {
		const folded = !here().timeline;
		untrack(() => timeline.fold(folded));
	});
	$effect(() => {
		const open = !timeline.folded;
		untrack(() => setTimeline(open));
	});

	followSelection();
	onMount(() => {
		installTooltips();
		loadChapters().catch(fail);
	});

	// 1–6, T, N – only outside text fields and dialogs, without modifiers (Blender-like; owner 2026-10-19)
	const typing = (el) => !!el?.closest?.('input, textarea, select, [contenteditable]:not([contenteditable="false"])');
	function keys(e) {
		if (e.ctrlKey || e.metaKey || e.altKey || e.repeat || e.defaultPrevented || typing(document.activeElement) || document.querySelector('dialog[open]')) return;
		const n = Number(e.key);
		if (n >= 1 && n <= STEPS.length) setStep(STEPS[n - 1].id);
		else if (e.key === 't' || e.key === 'T') window.dispatchEvent(new KeyboardEvent('keydown', { key: '\\', code: 'Backslash', ctrlKey: true }));
		else if (e.key === 'n' || e.key === 'N') foldPanels();
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
{#snippet shotBody()}<StudioPanel title={titleOf(PANELS[0])} icon="sliders" onclose={() => setOpen('shot', false)} tabs={shotTabs} testid="panel-shot"><ShotPanel tab={here().tab} /></StudioPanel>{/snippet}
{#snippet scriptBody()}<StudioPanel title={titleOf(PANELS[1])} icon="file-text" onclose={() => setOpen('script', false)}><ScriptPanel /></StudioPanel>{/snippet}
{#snippet handheldBody()}<StudioPanel title={titleOf(PANELS[2])} icon="smartphone" onclose={() => setOpen('handheld', false)}><HandheldPanel /></StudioPanel>{/snippet}
{#snippet generateBody()}<StudioPanel title={titleOf(PANELS[3])} icon="zap" onclose={() => setOpen('generate', false)}><GeneratePanel /></StudioPanel>{/snippet}
{#snippet promptBody()}<StudioPanel title={titleOf(PANELS[4])} icon="send" onclose={() => setOpen('prompt', false)}><PromptPanel /></StudioPanel>{/snippet}
{#snippet variantsBody()}<StudioPanel title={titleOf(PANELS[5])} icon="layers" onclose={() => setOpen('variants', false)}><VariantsPanel /></StudioPanel>{/snippet}
{#snippet outputBody()}<StudioPanel title="Output" icon="download" onclose={() => setOpen('output', false)}><OutputPanel /></StudioPanel>{/snippet}

{#snippet bottom()}
	<Timeline step={steps.current} onfold={() => timeline.fold(true)} />
{/snippet}

<div class="studio sc-tool">
	<AppShell mode="fill" minContent={24} resizeKey="storyboard-studio:shots" foldable railLabel="Shots" sidebarLabel="Shots" sidebarTag="aside" dockLabel="Shot panels" pageLabel="3D view" {bottom} bottomSize={timeline} bottomLabel="Timeline" bottomIcon="film" bottomResizeLabel="Resize the timeline" bottomUnfoldLabel="Show the timeline">
		{#snippet topbar()}
			<TopBar />
		{/snippet}
		{#snippet sidebar()}
			<ShotList />
		{/snippet}
		<Viewport />
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
