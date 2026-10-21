<script lang="ts">
	// The chapter's Storyboard page (#123; STUDIO-IN-CREATOR §4 N5 – the storyboard studio in the creator, its layout as
	// the owner approved it: workflows/storyboard-studio README "The layout", DESIGN "Steps and the timeline"). One layout
	// that always looks the same: the shots in the sidebar (the page's own list), the current shot in the main view
	// (shot-canvas – the 3D view replaces it in #124), the step's panels in the right dock (each with the creator's head).
	// The steps (keys 1–6) remember their panels and the inspector's tab; T folds the shots, N the step's panels, ← → the
	// previous / next shot. The document `storyboard:<toon>:<chapter>` saves, autosaves, recovers and refuses a stale
	// save like every creator page (MetaPage); its check is the creator's Problems panel (a row picks its shot).
	import { onDestroy, onMount, untrack, type Snippet } from 'svelte';
	import { replaceState } from '$app/navigation';
	import { page } from '$app/state';
	import { Icon, panelHeight, Segmented, useDock } from '@scutoons/ui';
	import type { Problem } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { dockWidth, getCreatorNav, usePageNav } from '$lib/nav.svelte.ts';
	import { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import { splitMessage } from '$lib/editor/storyboard-shots.ts';
	import { PANELS, READY_STEPS, SHOT_TABS, STEPS, stepOfKey, type PanelKind } from '$lib/editor/storyboard-steps.ts';
	import MetaPage from '$lib/components/page/meta-page.svelte';
	import StoryboardPanel from '$lib/components/storyboard/storyboard-panel.svelte';
	import ShotList from '$lib/components/storyboard/shot-list.svelte';
	import ShotInspector from '$lib/components/storyboard/shot-inspector.svelte';
	import ScriptPanel from '$lib/components/storyboard/script-panel.svelte';
	import RhythmPanel from '$lib/components/storyboard/rhythm-panel.svelte';
	import GeneratePanel from '$lib/components/storyboard/generate-panel.svelte';
	import MainView from '$lib/components/storyboard/main-view.svelte';
	import HandheldPanel from '$lib/components/storyboard/handheld-panel.svelte';
	import OutputPanel from '$lib/components/storyboard/output-panel.svelte';
	import StoryboardTimeline from '$lib/components/storyboard/storyboard-timeline.svelte';
	import ClipView from '$lib/components/storyboard/clip-view.svelte';
	import WorkspaceField from '$lib/components/storyboard/workspace-field.svelte';

	const nav = getCreatorNav();
	let { toon, chapterNumber, source }: { toon: string; chapterNumber: number; source: string } = $props();

	// the page is keyed by toon + chapter (the route remounts it)
	// svelte-ignore state_referenced_locally
	const sb = new StoryboardPage(toon, chapterNumber, source, () => nav.chapterSeason(toon, chapterNumber));
	onMount(() => void sb.load());
	// the current shot in the address (a link to it – the MCP's drafts, a colleague – opens it)
	$effect(() => {
		const id = sb.current;
		if (!id || page.url.searchParams.get('shot') === id) return;
		const url = new URL(page.url);
		url.searchParams.set('shot', id);
		replaceState(url, page.state);
	});

	const PANEL_WORD: Record<PanelKind, () => string> = {
		shot: () => m['storyboard.panel_shot'](),
		script: () => m['storyboard.panel_script'](),
		rhythm: () => m['storyboard.panel_rhythm'](),
		generate: () => m['storyboard.panel_generate'](),
		handheld: () => m['storyboard.panel_handheld'](),
		output: () => m['storyboard.panel_output']()
	};
	const STEP_WORD: Record<string, [() => string, () => string]> = {
		script: [() => m['storyboard.step_script'](), () => m['storyboard.step_script_hint']()],
		shots: [() => m['storyboard.step_shots'](), () => m['storyboard.step_shots_hint']()],
		layout: [() => m['storyboard.step_layout'](), () => m['storyboard.step_layout_hint']()],
		generate: [() => m['storyboard.step_generate'](), () => m['storyboard.step_generate_hint']()],
		handheld: [() => m['storyboard.step_handheld'](), () => m['storyboard.step_handheld_hint']()],
		export: [() => m['storyboard.step_export'](), () => m['storyboard.step_export_hint']()]
	};
	const TAB_WORD = {
		shot: [() => m['storyboard.tab_shot'](), () => m['storyboard.tab_shot_hint']()],
		camera: [() => m['storyboard.tab_camera'](), () => m['storyboard.tab_camera_hint']()],
		light: [() => m['storyboard.tab_light'](), () => m['storyboard.tab_light']()],
		figures: [() => m['storyboard.tab_figures'](), () => m['storyboard.tab_figures']()]
	} as const;

	// every step's own panel widths (remembered per step and kind, sc-panel-width:storyboard-<step>-<kind>)
	const sizes = Object.fromEntries(STEPS.map((s) => [s.id, Object.fromEntries(PANELS.map((p) => [p.kind, dockWidth(`storyboard-${s.id}-${p.kind}`, p.rem)]))]));
	const bodies: Record<PanelKind, Snippet> = { shot: shotBody, script: scriptBody, rhythm: rhythmBody, generate: generateBody, handheld: handheldBody, output: outputBody };
	/** the Shot panel names the shot (• = unsaved changes of the page) */
	const titleOf = (kind: PanelKind) => (kind === 'shot' && sb.sel ? `${PANEL_WORD.shot()} · ${sb.sel.id}` : PANEL_WORD[kind]());
	// the squeeze floor (DOCKING "Running out of room"): the compact inspector's rows need about 18rem
	PANELS.forEach((p, order) =>
		useDock(() =>
			sb.isOpen(p.kind) && (sb.sel || p.kind === 'rhythm')
				? { kind: `storyboard-${p.kind}`, order, min: p.kind === 'shot' ? 18 : undefined, size: sizes[sb.step][p.kind], title: titleOf(p.kind), icon: p.icon, close: () => sb.setOpen(p.kind, false), panel: bodies[p.kind] }
				: null
		)
	);

	// the timeline along the bottom (#126): one height for every step, each step its own fold (TimelineState)
	const timelineSize = panelHeight('sc-panel-height:storyboard-timeline', 10);
	usePageNav({
		sidebar: shotsNav,
		fill: true,
		extra: () => [{ href: `/${toon}/chapter/${chapterNumber}/storyboard`, label: m['storyboard.title']() }],
		bottom: () => (sb.data ? { panel: timelineBody, size: timelineSize, label: m['storyboard.tl_title'](), icon: 'film', resizeLabel: m['storyboard.tl_resize'](), unfoldLabel: m['storyboard.tl_unfold']() } : null)
	});
	sb.tl.follow();
	// a step shows its timeline folded or open; a fold by hand (the button, the rail, a drag) is the step's
	$effect(() => {
		const folded = sb.tl.here.folded;
		untrack(() => timelineSize.folded !== folded && timelineSize.fold(folded));
	});
	$effect(() => {
		const folded = timelineSize.folded;
		untrack(() => sb.tl.setFolded(folded));
	});
	// the phone sees the current shot and whether its camera is on
	$effect(() => {
		void [sb.current, sb.sel?.shot.takes?.on];
		untrack(() => sb.hh.sendState());
	});
	onDestroy(() => sb.hh.unpair());

	// the storyboard check (validateChapter) in the creator's Problems panel: "scene-25 sb-25-3: …" → place + text
	const validate = (): Problem[] => {
		const file = sb.doc.entity;
		const of = (level: 'error' | 'warning') => (message: string): Problem => {
			const { place, text } = splitMessage(message);
			return { level, file, locale: null, path: place, message: text };
		};
		return [...sb.problems.errors.map(of('error')), ...sb.problems.warnings.map(of('warning'))];
	};
	const gotoProblem = (p: Problem) => {
		const id = p.path.map(String).find((x) => sb.order.includes(x));
		if (id) sb.pick(id);
	};

	/** N: the step's panels fold to their rails – or come back; none open: the step's own set opens */
	function foldPanels() {
		const open = PANELS.filter((p) => sb.isOpen(p.kind));
		if (!open.length) return STEPS.find((s) => s.id === sb.step)?.panels.forEach((k) => sb.setOpen(k, true));
		const fold = open.some((p) => !sizes[sb.step][p.kind].folded);
		open.forEach((p) => sizes[sb.step][p.kind].fold(fold));
	}
	// 1–6, T, N, ← → – only outside text fields, controls with their own arrows and dialogs, without modifiers (Blender-like)
	const typing = (el: Element | null) => !!el?.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="spinbutton"], [role="radiogroup"], .sc-seg');
	function onkeydown(e: KeyboardEvent) {
		if (e.ctrlKey || e.metaKey || e.altKey || e.repeat || e.defaultPrevented || typing(document.activeElement) || document.querySelector('dialog[open]')) return;
		const step = stepOfKey(e.key);
		if (step) sb.setStep(step.id);
		else if (e.key === 't' || e.key === 'T') window.dispatchEvent(new KeyboardEvent('keydown', { key: '\\', code: 'Backslash', ctrlKey: true }));
		else if (e.key === 'n' || e.key === 'N') foldPanels();
		else if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') sb.move(e.key === 'ArrowLeft' ? -1 : 1);
		// I / O: the take under the playhead trimmed at the playhead (Take mode, #126 – research/TIMELINE.md §4)
		else if ((e.key === 'i' || e.key === 'I' || e.key === 'o' || e.key === 'O') && sb.tl.trimAtPlayhead(e.key.toLowerCase() === 'i' ? 'in' : 'out')) void 0;
		else return;
		e.preventDefault();
	}
	const position = $derived(sb.order.indexOf(sb.current ?? '') + 1);
</script>

<svelte:window {onkeydown} />

{#snippet shotsNav()}<ShotList {sb} />{/snippet}
{#snippet shotTabs()}
	<Segmented label={m['storyboard.shot_tabs']()} stretch radio options={SHOT_TABS.map((t) => ({ value: t, label: TAB_WORD[t][0](), title: TAB_WORD[t][1]() }))} value={sb.layout.tab} onchange={(t) => sb.setTab(t)} testid="shot-tabs" />
{/snippet}
{#snippet shotBody()}<StoryboardPanel title={titleOf('shot')} icon="sliders" onclose={() => sb.setOpen('shot', false)} tabs={shotTabs} testid="panel-shot"><ShotInspector {sb} tab={sb.layout.tab} /></StoryboardPanel>{/snippet}
{#snippet scriptBody()}<StoryboardPanel title={titleOf('script')} icon="file-text" onclose={() => sb.setOpen('script', false)} testid="panel-script"><ScriptPanel {sb} /></StoryboardPanel>{/snippet}
{#snippet rhythmBody()}<StoryboardPanel title={titleOf('rhythm')} icon="activity" onclose={() => sb.setOpen('rhythm', false)} testid="panel-rhythm"><RhythmPanel {sb} /></StoryboardPanel>{/snippet}
{#snippet handheldBody()}<StoryboardPanel title={titleOf('handheld')} icon="smartphone" onclose={() => sb.setOpen('handheld', false)} testid="panel-handheld"><HandheldPanel {sb} /></StoryboardPanel>{/snippet}
{#snippet outputBody()}<StoryboardPanel title={titleOf('output')} icon="download" onclose={() => sb.setOpen('output', false)} testid="panel-output"><OutputPanel {sb} /></StoryboardPanel>{/snippet}
{#snippet timelineBody()}<StoryboardTimeline {sb} onfold={() => timelineSize.fold(true)} />{/snippet}
{#snippet generateBody()}<StoryboardPanel title={titleOf('generate')} icon="zap" onclose={() => sb.setOpen('generate', false)} testid="panel-generate"><GeneratePanel {sb} /></StoryboardPanel>{/snippet}

{#snippet stepper()}
	{#if sb.order.length}
		<span class="stepper">
			<button type="button" class="sc-round" aria-label={m['storyboard.prev_shot']()} title={m['storyboard.prev_shot']()} disabled={position <= 1} onclick={() => sb.move(-1)}><Icon name="chevron-left" /></button>
			<span class="count">{m['storyboard.position']({ n: position, total: sb.order.length })}</span>
			<button type="button" class="sc-round" aria-label={m['storyboard.next_shot']()} title={m['storyboard.next_shot']()} disabled={position >= sb.order.length} onclick={() => sb.move(1)}><Icon name="chevron-right" /></button>
		</span>
	{/if}
{/snippet}

<MetaPage {toon} title={m['storyboard.title']()} docs={[sb.doc]} {validate} ongoto={gotoProblem} fill wide help="storyboard" actions={stepper}>
	{#if !sb.data}
		<div class="missing">
			<p class="sc-empty">{m['storyboard.missing']()}</p>
			<button type="button" onclick={() => sb.create()} data-testid="storyboard-create">{m['storyboard.create']()}</button>
		</div>
	{:else}
		<div class="page" data-testid="storyboard-page">
			<div class="bar">
				<div class="sc-seg steps" role="group" aria-label={m['storyboard.steps']()} data-testid="storyboard-steps">
					{#each READY_STEPS as s (s.id)}
						<button type="button" aria-pressed={sb.step === s.id} title="{STEP_WORD[s.id][1]()} ({s.key})" onclick={() => sb.setStep(s.id)} data-testid="step-{s.id}"><kbd>{s.key}</kbd>{STEP_WORD[s.id][0]()}</button>
					{/each}
				</div>
				<div class="sc-seg" role="group" aria-label={m['storyboard.panels']()} title={m['storyboard.panels_hint']()}>
					{#each PANELS as p (p.kind)}
						<button type="button" class="is-square" aria-pressed={sb.isOpen(p.kind)} aria-label={PANEL_WORD[p.kind]()} title={PANEL_WORD[p.kind]()} onclick={() => sb.setOpen(p.kind, !sb.isOpen(p.kind))} data-testid="toggle-{p.kind}"><Icon name={p.icon} /></button>
					{/each}
				</div>
				<span class="grow"></span>
				<WorkspaceField {sb} />
			</div>
			<div class="view"><MainView {sb} /><ClipView {sb} /></div>
		</div>
	{/if}
</MetaPage>

<style>
	.page {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
	}
	/* the step row: the steps, the step's panels, the workspace folder – it wraps on narrow screens */
	.bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05);
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
		flex: none;
		container-type: inline-size;
	}
	.steps kbd {
		margin-right: var(--space-05);
		font: inherit;
		opacity: 0.6;
	}
	.grow {
		flex: 1;
	}
	.view {
		position: relative;
		flex: 1 1 auto;
		min-height: 0;
	}
	.missing {
		display: grid;
		justify-items: start;
		gap: var(--space-1);
		padding: var(--space-1);
	}
	.stepper {
		display: inline-flex;
		align-items: center;
		gap: var(--space-025);
	}
	.count {
		min-width: 3.5rem;
		text-align: center;
		font-variant-numeric: tabular-nums;
	}
	/* a narrow main (panels open, phones): the steps' numbers go, the words stay – and wrap rather than overflow */
	@container (max-width: 30rem) {
		.steps kbd {
			display: none;
		}
	}
	.steps {
		flex-wrap: wrap;
	}
</style>
