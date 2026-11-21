<script lang="ts">
	import { Button } from '@scutoons/ui';
	import { Section } from '@scutoons/ui/editor';
	import { experienceOutline } from '$lib/editor/experiences.ts';
	import { openRequest, readStageEvent, STAGE_FRAME_PATH, STAGE_LOADING, STAGE_SANDBOX, STAGE_VIEWS, stageAfter, stageMessage, type StageView } from '$lib/editor/xr-stage-frame.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';

	/**
	 * The stage: one scene of the experience played on a desk, without a camera. The engine and the experience's plugins
	 * run in the sandboxed frame (lib/editor/xr-stage-frame.ts); this is its panel. View only.
	 */
	let { content, assets }: { content: unknown; assets: string } = $props();

	const scenes = $derived(experienceOutline((content as { configuration?: unknown } | null)?.configuration).scenes);
	let picked = $state('');
	const scene = $derived(scenes.find((s) => s.id === picked)?.id ?? scenes[0]?.id ?? '');
	let run = $state(0);
	let view = $state<StageView>('reader');
	let frame = $state<HTMLIFrameElement>();
	let shown = $state(STAGE_LOADING);

	const viewName: Record<StageView, () => string> = {
		reader: m['creator.experiences_stage_view_reader'],
		top: m['creator.experiences_stage_view_top'],
		low: m['creator.experiences_stage_view_low'],
		far: m['creator.experiences_stage_view_far']
	};

	function onMessage(event: MessageEvent) {
		const target = frame?.contentWindow;
		if (!target || event.source !== target) return;
		const e = readStageEvent(event.data);
		if (!e) return;
		if (e.kind !== 'ready') return void (shown = stageAfter(shown, e));
		// a frame loaded anew: it starts in the reader's view and takes the experience as it is now
		view = 'reader';
		const request = openRequest($state.snapshot(content), scene, asset(assets), getLocale());
		shown = request ? STAGE_LOADING : { ...STAGE_LOADING, phase: 'failed' };
		// the frame's origin is opaque, so it cannot be named; what goes there is the experience it is to play
		if (request) target.postMessage(stageMessage(request), '*');
	}

	function look(next: StageView) {
		view = next;
		frame?.contentWindow?.postMessage(stageMessage({ kind: 'view', view: next }), '*');
	}
</script>

<svelte:window onmessage={onMessage} />

<Section id="xr-stage" label={m['creator.experiences_stage']()} open>
	{#if !scene}
		<p class="sc-empty">{m['creator.experiences_stage_none']()}</p>
	{:else}
		<div class="bar">
			{#if scenes.length > 1}
				<select value={scene} onchange={(e) => (picked = e.currentTarget.value)} aria-label={m['creator.experiences_stage_scene']()} data-testid="experience-stage-scene">
					{#each scenes as s (s.id)}<option value={s.id}>{s.label || s.id}</option>{/each}
				</select>
			{/if}
			<span class="views" role="group" aria-label={m['creator.experiences_stage_view']()}>
				{#each STAGE_VIEWS as v (v)}
					<Button variant="secondary" size="sm" aria-pressed={view === v} disabled={shown.phase !== 'playing'} onclick={() => look(v)} data-testid="experience-stage-view-{v}">{viewName[v]()}</Button>
				{/each}
			</span>
			<Button variant="secondary" size="sm" onclick={() => run++} data-testid="experience-stage-replay">{m['creator.experiences_stage_replay']()}</Button>
		</div>
		<div class="stage" data-testid="experience-stage" data-phase={shown.phase}>
			{#key `${assets}|${scene}|${run}`}
				<iframe bind:this={frame} src={STAGE_FRAME_PATH} sandbox={STAGE_SANDBOX} title={m['creator.experiences_stage_frame']()}></iframe>
			{/key}
			{#if shown.phase === 'loading'}
				<p class="note" role="status">{m['creator.experiences_stage_loading']()}</p>
			{:else if shown.phase === 'failed'}
				<div class="note" role="alert" data-testid="experience-stage-failed">
					<p>{m['creator.experiences_stage_failed']()}</p>
					{#if shown.problems.length}<ul>{#each shown.problems as p, i (i)}<li>{p}</li>{/each}</ul>{/if}
				</div>
			{/if}
			{#if shown.subtitle}<p class="subtitle">{shown.subtitle}</p>{/if}
		</div>
		<p class="hint">{m['creator.experiences_stage_hint']()}</p>
	{/if}
</Section>

<style>
	.bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem 1rem;
		margin-bottom: 0.5rem;
	}
	.views {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
	.views :global([aria-pressed='true']) {
		border-color: var(--ink);
	}
	.stage {
		position: relative;
		aspect-ratio: 16 / 10;
		max-height: 70vh;
		width: 100%;
		border: var(--border-width-regular) solid var(--border-subtle);
		background: var(--surface-panel);
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
		inset: auto 0.5rem 0.5rem;
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
	div.note {
		border-color: var(--state-error);
		color: var(--state-error);
	}
	.subtitle {
		position: absolute;
		inset: auto 0.5rem 0.5rem;
		margin: 0;
		text-align: center;
		pointer-events: none;
	}
	.hint {
		margin: 0.5rem 0 0;
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
</style>
