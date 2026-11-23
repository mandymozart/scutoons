<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import type { XrStage } from '@scutoons/xr-engine/stage';
	import { STAGE_FILES_PATH, STAGE_LOADING, stageAfter, stageInput, type StagePlacement, type StageRef, type StageShown, type StageTool, type StageView } from '$lib/editor/xr-stage.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';

	/**
	 * The workspace's stage: one scene of the experience on the desk, filling its parent – the engine and the
	 * experience's plugins run in the creator's page (owner 2026-11-21: no sandbox). A new `scene` or `run` opens the
	 * stage anew; `view`, `tool`, `selected` and `edited` reach the stage that plays; a click reports what it picked, a
	 * gizmo drag the node's placement.
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
		moved,
		tool,
		handles,
		onplace,
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
		edited: { at: number; trees: Record<string, object | null> } | null;
		/** a target's place changed: the stage lays it out there */
		moved: { at: number; target: string; place: { surface?: string; x: number; y: number; width: number; height?: number } } | null;
		tool: StageTool;
		/** the gizmo's handles show even with select (the setting) */
		handles: boolean;
		onplace: (ref: StageRef, placement: StagePlacement) => void;
		shown?: StageShown;
	} = $props();

	let host = $state<HTMLDivElement>();
	let stage = $state.raw<XrStage | null>(null);
	let opening = 0;

	async function open(container: HTMLDivElement, key: number) {
		shown = STAGE_LOADING;
		const input = stageInput($state.snapshot(content));
		if (!input) return void (shown = { ...STAGE_LOADING, phase: 'failed' });
		try {
			const { openStage } = await import('@scutoons/xr-engine/stage');
			const opened = await openStage(container, input.configuration, input.plugins, scene, {
				assetsBase: asset(assets),
				stageBase: STAGE_FILES_PATH,
				language: getLocale,
				handlers: {
					status: (status, message) => (shown = stageAfter(shown, status, message ?? null)),
					found: () => {},
					subtitle: (text) => (shown = { ...shown, subtitle: text }),
					picked: (ref) => onpick(ref ? { target: ref.target, path: ref.path ? [...ref.path] : null } : null),
					placed: (ref, placement) => onplace({ target: ref.target, path: ref.path ? [...ref.path] : null }, placement)
				}
			});
			if ('problems' in opened) return void (shown = { ...STAGE_LOADING, phase: 'failed', problems: opened.problems });
			if (key !== opening) return void opened.stage.dispose();
			stage = opened.stage;
		} catch (e) {
			console.error('[xr-stage]', e);
			shown = { ...STAGE_LOADING, phase: 'failed', problems: [e instanceof Error ? e.message : String(e)] };
		}
	}

	// a new experience folder, scene or replay: the stage opens anew in an emptied element
	$effect(() => {
		void `${assets}|${scene}|${run}`;
		const container = host;
		if (!container) return;
		const key = ++opening;
		untrack(() => void open(container, key));
		return () => {
			const old = stage;
			stage = null;
			void old?.dispose();
			container.replaceChildren();
		};
	});

	$effect(() => {
		if (stage && shown.phase === 'playing') stage.view(view);
	});
	$effect(() => {
		if (stage && shown.phase === 'playing') stage.tool(tool, handles);
	});
	$effect(() => {
		const ref = selected ? { target: selected.target, path: selected.path ? [...selected.path] : null } : null;
		if (stage && shown.phase === 'playing') stage.select(ref);
	});
	$effect(() => {
		const edit = edited;
		if (!edit || !stage || shown.phase !== 'playing') return;
		for (const [target, entity] of Object.entries(edit.trees)) stage.override(target, entity);
	});
	$effect(() => {
		const move = moved;
		if (move && stage && shown.phase === 'playing') stage.place(move.target, move.place);
	});

	onMount(() => () => {
		opening++;
		void stage?.dispose();
	});
</script>

<div class="stage" data-testid="experience-stage" data-phase={shown.phase}>
	<div class="host" bind:this={host}></div>
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
		background: var(--tool-ground);
	}
	.host {
		position: absolute;
		inset: 0;
		overflow: hidden;
	}
	.host :global(canvas) {
		display: block;
		width: 100%;
		height: 100%;
	}
	.note {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		max-width: 28rem;
		margin: 0;
		padding: 0.5rem 0.75rem;
		background: var(--tool-popover);
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
