<script lang="ts">
	// The 3D view (#124; owner 2026-10-09/10: "a free moving camera in a 3d view"; 2026-10-20: "of course threejs, it is
	// the main tool of the storyboard"; workflows/storyboard-studio README "The layout"): the current shot's scene with
	// the frames' own renderer (lib/storyboard/render), live. Its tool row (the figure tool Move / Turn, the texts, "Shot
	// camera", the hints) and the stage; the camera controls, the figure gestures, the gizmo and the texts are their own
	// components on the shared Stage3d. Every write goes through the page's editShot – one undo step per gesture, the
	// dirty dot and autosave as every edit. A change the view wrote itself doesn't rebuild the scene (it is already so);
	// any other change of the shot (a field, undo, another shot) rebuilds it – the old scene freed first.
	import { onMount, untrack, type Snippet } from 'svelte';
	import { base } from '$app/paths';
	import { Segmented } from '@scutoons/ui';
	import { shotSpec, type Shot } from '@scutoons/content/storyboard';
	import { m } from '$lib/paraglide/messages.js';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import { Stage3d } from '$lib/storyboard/stage.svelte.ts';
	import { personaModelUrls } from '$lib/storyboard/persona-models.ts';
	import { loadFonts } from '$lib/storyboard/render/index.ts';
	import View3dOrbit from '$lib/components/storyboard/view-3d-orbit.svelte';
	import View3dFigures from '$lib/components/storyboard/view-3d-figures.svelte';
	import View3dGizmo from '$lib/components/storyboard/view-3d-gizmo.svelte';
	import View3dTexts from '$lib/components/storyboard/view-3d-texts.svelte';

	// `lead`: the main view's editor switch at the start of the tool row (#125)
	let { sb, lead }: { sb: StoryboardPage; lead?: Snippet } = $props();

	const stage = new Stage3d(personaModelUrls(base));
	let host = $state<HTMLDivElement>();
	let frame = $state<HTMLDivElement>();
	let tool = $state<'move' | 'turn'>('move');
	let texts = $state(true);
	/** the picked figure (its gizmo), per shot */
	let figure = $state<number | null>(null);

	const specOf = () => (sb.sel ? shotSpec(sb.sel.scene, $state.snapshot(sb.sel.shot) as Shot, 1) : null);
	/** the spec the scene shows (or the view wrote itself – the scene already shows it) */
	let lastKey = '';
	let shotId = '';
	$effect(() => {
		const spec = specOf();
		if (!spec) return;
		const key = JSON.stringify(spec);
		untrack(() => {
			if (spec.id !== shotId) {
				shotId = spec.id;
				figure = null;
			}
			if (key === lastKey) return;
			lastKey = key;
			void stage.build(spec).then(() => fit());
		});
	});

	/** one gesture's write: one undo step (none when it changes nothing – a click on the gizmo); the scene stays as it is */
	function commit(change: (shot: Shot) => void) {
		if (!sb.sel) return;
		const before = $state.snapshot(sb.sel.shot) as Shot;
		const probe = structuredClone(before);
		change(probe);
		if (JSON.stringify(probe) === JSON.stringify(before)) return;
		sb.editShot(change);
		const spec = specOf();
		lastKey = spec ? JSON.stringify(spec) : '';
		stage.draw();
	}

	function fit() {
		if (!host) return;
		stage.fit(host.clientWidth, host.clientHeight);
		stage.draw();
	}
	onMount(() => {
		frame?.prepend(stage.renderer.domElement);
		void loadFonts().then(() => stage.draw());
		const ro = new ResizeObserver(fit);
		if (host) ro.observe(host);
		// for checks in the browser (dev only): the stage
		if (import.meta.env.DEV) (window as unknown as { __storyboard3d?: Stage3d }).__storyboard3d = stage;
		// the handheld camera's live pose and a take's moment hold this view's camera (#126)
		sb.hh.attach(stage);
		return () => {
			sb.hh.detach(stage);
			ro.disconnect();
			stage.dispose();
		};
	});

	/** back to the camera the shot's size / angle give: only a hand camera is removed (nothing unsaved for nothing) */
	function shotCamera() {
		if (!sb.sel?.shot.camera) return;
		sb.editShot((shot) => void delete shot.camera);
	}
	const gesture = $derived(tool === 'turn' ? m['storyboard.view3d_gesture_turn']() : m['storyboard.view3d_gesture_move']());
</script>

<section class="view" aria-label={sb.sel ? m['storyboard.view3d_label']({ id: sb.sel.id }) : undefined} data-testid="shot-3d-view">
	<div class="tools">
		{@render lead?.()}
		<Segmented
			label={m['storyboard.view3d_tool']()}
			options={[
				{ value: 'move', label: m['storyboard.view3d_move'](), icon: 'move' },
				{ value: 'turn', label: m['storyboard.view3d_turn'](), icon: 'rotate-cw' }
			]}
			value={tool}
			onchange={(v) => (tool = v)}
			testid="view3d-tool"
		/>
		<Segmented label={m['storyboard.view3d_texts']()} options={[{ value: 'on', label: m['storyboard.view3d_texts'](), icon: 'type', title: m['storyboard.view3d_texts_hint']() }]} value={texts ? 'on' : null} onchange={() => (texts = true)} onclear={() => (texts = false)} testid="view3d-texts" />
		<button type="button" onclick={shotCamera} disabled={!sb.sel?.shot.camera} title={m['storyboard.view3d_shot_camera_hint']()} data-testid="view3d-shot-camera">{m['storyboard.view3d_shot_camera']()}</button>
		{#if stage.busy}<span class="sc-chip" role="status">{m['storyboard.view3d_rendering']()}</span>{/if}
		{#if sb.sel?.shot.camera}<span class="sc-chip" title={m['storyboard.view3d_hand_hint']()} data-testid="view3d-hand">{m['storyboard.view3d_hand']()}</span>{/if}
		<span class="hint sc-caption" title={m['storyboard.view3d_hint']({ gesture })}><span>{m['storyboard.view3d_hint_short']({ gesture })}</span></span>
	</div>
	<div class="stage" bind:this={host}>
		<div class="frame" bind:this={frame} class:hidden={!sb.sel}>
			<View3dTexts {stage} {texts} />
		</div>
		{#if stage.error}<p class="sc-error">{m['storyboard.view3d_error']({ message: stage.error })}</p>{/if}
	</div>
	<View3dOrbit {stage} {sb} {commit} />
	<View3dFigures {stage} {sb} {tool} bind:figure {commit} />
	<View3dGizmo {stage} {tool} {figure} {commit} />
</section>

<style>
	.view {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		min-width: 0;
	}
	.tools {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding: var(--space-025) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
		flex-wrap: wrap;
	}
	/* one line in the room the tools leave, the full text in its tooltip; where that room is narrower than the line it
	   goes as a whole instead of being cut mid-word (the studio's STUDIO-LAYOUT §1 problem 6) */
	.hint {
		flex: 1 1 0;
		min-width: 0;
		container-type: inline-size;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	@container (max-width: 30rem) {
		.hint > span {
			display: none;
		}
	}
	.stage {
		flex: 1;
		min-height: 0;
		position: relative;
		background: var(--ink);
		overflow: hidden;
		touch-action: none;
	}
	/* out of the flow: the stage's size comes from the layout, the frame is fitted into it (Stage3d.fit) */
	.frame {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		line-height: 0;
	}
	.frame.hidden {
		display: none;
	}
	.frame :global(canvas.gl) {
		display: block;
	}
	.stage .sc-error {
		position: absolute;
		bottom: var(--space-1);
		left: var(--space-1);
		background: var(--paper);
	}
</style>
