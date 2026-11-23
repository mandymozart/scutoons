<script lang="ts">
	import { ToolGroup } from '@scutoons/ui/editor';
	import { contentUrl, type OutlineScene } from '$lib/editor/experiences.ts';
	import { STAGE_TOOLS, STAGE_VIEWS, type StageTool, type StageView } from '$lib/editor/xr-stage.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';

	/**
	 * The Scenes tab: the stage's view and Play again over a strip of the experience's scenes (each with its first
	 * target's picture). Picking a scene plays it on the stage.
	 */
	let {
		scenes,
		current,
		assets,
		view = $bindable(),
		tool = $bindable(),
		playing,
		onpick,
		onreplay
	}: { scenes: OutlineScene[]; current: string; assets: string; view: StageView; tool: StageTool; playing: boolean; onpick: (id: string) => void; onreplay: () => void } = $props();

	const viewName: Record<StageView, () => string> = {
		reader: m['creator.experiences_stage_view_reader'],
		top: m['creator.experiences_stage_view_top'],
		low: m['creator.experiences_stage_view_low'],
		far: m['creator.experiences_stage_view_far']
	};
	const ICONS: Record<StageView, string> = { reader: 'smartphone', top: 'arrow-down', low: 'minus', far: 'maximize' };
	const views = STAGE_VIEWS.map((v) => ({ value: v, name: viewName[v](), label: viewName[v](), icon: ICONS[v] }));
	const TOOL_ICONS: Record<StageTool, string> = { select: 'mouse-pointer', translate: 'move', rotate: 'rotate-cw', scale: 'maximize-2' };
	const toolName: Record<StageTool, () => string> = {
		select: m['creator.experiences_tool_select'],
		translate: m['creator.experiences_tool_move'],
		rotate: m['creator.experiences_tool_rotate'],
		scale: m['creator.experiences_tool_scale']
	};
	const tools = STAGE_TOOLS.map((t) => ({ value: t, name: toolName[t](), label: toolName[t](), icon: TOOL_ICONS[t] }));
	const picture = (s: OutlineScene) => {
		const entry = [...s.surfaces.flatMap((x) => x.entries), ...s.loose].find((e) => e.target?.image);
		return entry?.target ? contentUrl(assets, entry.target.image, asset) : null;
	};
</script>

<div class="scenes-tab">
	<div class="tools">
		<span class:idle={!playing} data-testid="experience-tools"><ToolGroup label={m['creator.experiences_stage_tool']()} options={tools} value={tool} expanded onchoose={(t) => (tool = t as StageTool)} /></span>
		<span class:idle={!playing}><ToolGroup label={m['creator.experiences_stage_view']()} options={views} value={view} expanded onchoose={(v) => (view = v as StageView)} /></span>
		<button type="button" class="replay" onclick={onreplay} data-testid="experience-stage-replay">{m['creator.experiences_stage_replay']()}</button>
	</div>
	<ul class="strip" data-testid="experience-scenes">
		{#each scenes as s (s.id)}
			{@const pic = picture(s)}
			<li>
				<button type="button" class="card" aria-pressed={s.id === current} onclick={() => onpick(s.id)} data-testid="experience-scene-{s.id}">
					{#if pic}<img src={pic} alt="" loading="lazy" />{:else}<span class="blank"></span>{/if}
					<span class="name">{s.label || s.id}</span>
				</button>
			</li>
		{/each}
	</ul>
</div>

<style>
	.scenes-tab {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		height: 100%;
		min-height: 0;
	}
	.tools {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem;
	}
	.idle {
		opacity: 0.5;
		pointer-events: none;
	}
	.replay {
		margin: 0;
	}
	.strip {
		display: flex;
		gap: 0.5rem;
		margin: 0;
		padding: 0 0 0.25rem;
		list-style: none;
		overflow-x: auto;
		min-height: 0;
		flex: 1;
	}
	.card {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		justify-content: flex-start;
		gap: 0;
		width: 9rem;
		height: 100%;
		min-height: 5rem;
		margin: 0;
		padding: 0;
		border: var(--border-width-regular) solid var(--border-subtle);
		background: var(--paper);
		color: inherit;
		text-align: left;
		cursor: pointer;
	}
	.card[aria-pressed='true'] {
		border-color: var(--ink);
		outline: var(--border-width-regular) solid var(--ink);
	}
	.card img,
	.blank {
		flex: 1;
		min-height: 0;
		width: 100%;
		object-fit: cover;
		background: var(--surface-panel);
	}
	.name {
		display: block;
		width: 100%;
		box-sizing: border-box;
		text-align: left;
		padding: 0.15rem 0.4rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
