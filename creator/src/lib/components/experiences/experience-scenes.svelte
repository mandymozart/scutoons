<script lang="ts">
	import { Button, Pill, ToolGroup } from '@scutoons/ui/editor';
	import { contentUrl, type OutlineScene } from '$lib/editor/experiences.ts';
	import { STAGE_TOOLS, STAGE_VIEWS, type StageTool, type StageView } from '$lib/editor/xr-stage.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import Filmstrip from '$lib/components/filmstrip/filmstrip.svelte';

	/**
	 * The Scenes tab: the stage's view and Play again over the Filmstrip of the experience's scenes (each with its first
	 * target's picture; lap 2 C3: the same strip as a storyboard's shots). Picking a scene plays it on the stage.
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
	const groups = $derived([{ key: 'scenes', label: '', frames: scenes.map((s) => ({ id: s.id, label: s.label || s.id, aspect: 1, src: picture(s) })) }]);
</script>

<div class="scenes-tab">
	<div class="tools">
		<span class:idle={!playing} data-testid="experience-tools"><Pill><ToolGroup label={m['creator.experiences_stage_tool']()} options={tools} value={tool} expanded onchoose={(t) => (tool = t as StageTool)} /></Pill></span>
		<span class:idle={!playing}><Pill><ToolGroup label={m['creator.experiences_stage_view']()} options={views} value={view} expanded onchoose={(v) => (view = v as StageView)} /></Pill></span>
		<Pill><Button variant="quiet" icon="rotate-ccw" onclick={onreplay} data-testid="experience-stage-replay">{m['creator.experiences_stage_replay']()}</Button></Pill>
	</div>
	<div class="strip"><Filmstrip {groups} {current} onpick={(id) => onpick(id)} label={m['shell.filmstrip_scenes']()} row testid="experience-scenes" /></div>
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
		gap: var(--kit-gap);
	}
	.idle {
		opacity: 0.5;
		pointer-events: none;
	}
	.strip {
		flex: 1;
		min-height: 0;
	}
</style>
