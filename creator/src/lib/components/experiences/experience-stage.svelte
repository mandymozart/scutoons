<script lang="ts">
	import { openRequest, readStageEvent, STAGE_FRAME_PATH, STAGE_LOADING, STAGE_SANDBOX, stageAfter, stageMessage, type StageRef, type StageShown, type StageView } from '$lib/editor/xr-stage-frame.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';

	/**
	 * The workspace's stage: one scene of the experience on the desk, filling its parent. The engine and the experience's
	 * plugins run in the sandboxed frame (lib/editor/xr-stage-frame.ts); this element only carries it and its notes.
	 * A new `scene` or `run` loads the frame anew; `view` turns the camera of the frame that plays; `selected` is outlined
	 * there, a click on the stage reports what it picked (`onpick`).
	 */
	let {
		content,
		assets,
		scene,
		view,
		run,
		selected,
		onpick,
		edited,
		shown = $bindable(STAGE_LOADING)
	}: {
		content: unknown;
		assets: string;
		scene: string;
		view: StageView;
		run: number;
		selected: StageRef | null;
		onpick: (ref: StageRef | null) => void;
		/** the last edit: the targets whose entity changed, with their trees as they are now */
		edited: { at: number; trees: Record<string, object> } | null;
		shown?: StageShown;
	} = $props();

	let frame = $state<HTMLIFrameElement>();

	function onMessage(event: MessageEvent) {
		const target = frame?.contentWindow;
		if (!target || event.source !== target) return;
		const e = readStageEvent(event.data);
		if (!e) return;
		if (e.kind === 'picked') return onpick(e.ref);
		if (e.kind !== 'ready') return void (shown = stageAfter(shown, e));
		// a frame loaded anew takes the experience as it is now
		const request = openRequest($state.snapshot(content), scene, asset(assets), getLocale());
		shown = request ? STAGE_LOADING : { ...STAGE_LOADING, phase: 'failed' };
		// the frame's origin is opaque, so it cannot be named; what goes there is the experience it is to play
		if (request) target.postMessage(stageMessage(request), '*');
	}

	$effect(() => {
		const next = view;
		if (shown.phase === 'playing') frame?.contentWindow?.postMessage(stageMessage({ kind: 'view', view: next }), '*');
	});
	$effect(() => {
		const edit = edited;
		if (!edit || shown.phase !== 'playing') return;
		for (const [target, entity] of Object.entries(edit.trees)) frame?.contentWindow?.postMessage(stageMessage({ kind: 'entity', target, entity }), '*');
	});
	$effect(() => {
		const ref = selected ? { target: selected.target, path: selected.path ? [...selected.path] : null } : null;
		if (shown.phase === 'playing') frame?.contentWindow?.postMessage(stageMessage({ kind: 'select', ref }), '*');
	});
</script>

<svelte:window onmessage={onMessage} />

<div class="stage" data-testid="experience-stage" data-phase={shown.phase}>
	{#key `${assets}|${scene}|${run}`}
		<iframe bind:this={frame} src={STAGE_FRAME_PATH} sandbox={STAGE_SANDBOX} title={m['creator.experiences_stage_frame']()}></iframe>
	{/key}
	{#if shown.phase === 'loading'}
		<p class="note" role="status">{m['creator.experiences_stage_loading']()}</p>
	{:else if shown.phase === 'failed'}
		<div class="note failed" role="alert" data-testid="experience-stage-failed">
			<p>{m['creator.experiences_stage_failed']()}</p>
			{#if shown.problems.length}<ul>{#each shown.problems as p, i (i)}<li>{p}</li>{/each}</ul>{/if}
		</div>
	{/if}
	{#if shown.subtitle}<p class="subtitle">{shown.subtitle}</p>{/if}
</div>

<style>
	.stage {
		position: absolute;
		inset: 0;
		background: var(--editor-stage, var(--ink));
	}
	iframe {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		border: 0;
	}
	.note {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		max-width: 28rem;
		margin: 0;
		padding: 0.5rem 0.75rem;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--border-subtle);
		overflow-wrap: anywhere;
	}
	.note p {
		margin: 0;
	}
	.note ul {
		margin: 0.25rem 0 0;
		padding-left: 1.25rem;
	}
	.failed {
		border-color: var(--state-error);
		color: var(--state-error);
	}
	.subtitle {
		position: absolute;
		inset: auto 1rem 30%;
		margin: 0;
		text-align: center;
		pointer-events: none;
	}
</style>
