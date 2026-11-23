<script lang="ts">
	import { ScrubNumber } from '@scutoons/ui';
	import UnitNumber from '$lib/components/fields/unit-number.svelte';
	import { readSettings, SETTINGS_KEY } from '$lib/shell/settings.ts';
	import KitStates from '$lib/components/page/kit-states.svelte';
	import { at, Button, Floats, FloatingPanels, ListRow, Pill, PillTabs, Section, Toggle, ToolGroup, ToolMenu, type PanelSpec, type TabSpec } from '@scutoons/ui/editor';

	// Dev only: the editor kit over a stage – drag a panel by its grip, a tab into another panel or out onto the stage,
	// resize an edge (the neighbours make room), fold by a double-click on the head; reload and the layout is back.
	// States: every component of the kit in its states (kit-states.svelte).
	const TABS: TabSpec[] = [
		{ id: 'outline', name: 'Outline', icon: 'list', about: 'The open item’s structure' },
		{ id: 'history', name: 'History', icon: 'clock', about: 'Saved versions' },
		{ id: 'inspector', name: 'Inspector', icon: 'sliders', about: 'Whatever is selected', essential: true },
		{ id: 'problems', name: 'Problems', icon: 'alert-triangle', about: 'Errors and warnings' },
		{ id: 'library', name: 'Library', icon: 'grid', about: 'Every asset of the scope', size: { w: 26, h: 18 } },
		{ id: 'wanted', name: 'Wanted', icon: 'image', about: 'What is missing' },
		{ id: 'info', name: 'Info', icon: 'info', about: 'The control under the pointer' }
	];
	const PANELS: PanelSpec[] = [
		{
			id: 'tools',
			name: 'Tools',
			icon: 'tool',
			about: 'The stage’s tools',
			place: at('top-left', 21, 0),
			size: { w: 'content', h: 'content' },
			min: { w: 3, h: 2 },
			inline: true,
			essential: true
		},
		{ id: 'outline', name: 'Outline', icon: 'list', about: '', place: at('top-left', 0, 0), size: { w: 20, h: 'fill' }, min: { w: 12, h: 8 }, tabs: ['outline', 'history'] },
		{ id: 'inspector', name: 'Inspector', icon: 'sliders', about: '', place: at('top-right', 0, 0), size: { w: 24, h: 'fill' }, min: { w: 16, h: 10 }, tabs: ['inspector', 'problems'] },
		{ id: 'library', name: 'Library', icon: 'grid', about: '', place: at('bottom-left', 21, 0), size: { w: 'fill', h: 16 }, min: { w: 20, h: 8 }, tabs: ['library', 'wanted', 'info'] }
	];
	const WORKSPACES = [
		{ id: 'scene', label: 'Scene' },
		{ id: 'character', label: 'Character' },
		{ id: 'states', label: 'States' }
	];

	let workspace = $state('scene');
	const floats = $derived(new Floats(`kit-${workspace === 'states' ? 'scene' : workspace}`, PANELS, TABS));
	// Day / Night only in the creator's Settings (Look): the demo follows what they keep in this browser
	let day = $state(false);
	$effect(() => {
		try {
			day = readSettings(JSON.parse(localStorage.getItem(SETTINGS_KEY) ?? 'null')).theme === 'day';
		} catch {
			day = false;
		}
	});

	let tool = $state('move');
	let pick = $state<string[]>(['entities']);
	let snap = $state(true);
	let grid = $state<number | undefined>(1);
	let x = $state<number | undefined>(120);
	let y = $state<number | undefined>(40);
	let scale = $state<number | undefined>(1);
	let fade = $state<number | undefined>(1.5);
	let slot = $state<number | undefined>(12.5);
	let visible = $state(true);
	let shadow = $state(false);
	let glow = $state(true);
	let strength = $state<number | undefined>(0.35);
</script>

<svelte:head><title>Editor kit</title></svelte:head>

<div class="kit sc-tool sc-desktop-only" data-theme={day ? 'day' : 'night'}>
	<header class="bar">
		<span class="name">Editor kit</span>
		<Pill as="nav" label="Workspaces"><PillTabs items={WORKSPACES} on={workspace} onselect={(id) => (workspace = id)} /></Pill>
		<span class="spacer"></span>
		{#if workspace !== 'states'}<Pill><Button variant="quiet" icon="rotate-ccw" onclick={() => floats.reset()}>Reset layout</Button></Pill>{/if}
	</header>
	<main class="room" class:scroll={workspace === 'states'}>
		{#if workspace === 'states'}
			<KitStates />
		{:else}
		{#key floats}
			<FloatingPanels {floats}>
				<div class="stage-art"><span>Stage · {WORKSPACES.find((w) => w.id === workspace)?.label}</span></div>
				{#snippet toolbar()}
					<div class="tools">
						<ToolGroup
							label="Transform"
							expanded
							value={tool}
							options={[
								{ value: 'move', name: 'Move', label: 'Move (W)', icon: 'move', key: 'W' },
								{ value: 'rotate', name: 'Rotate', label: 'Rotate (E)', icon: 'rotate-cw', key: 'E' },
								{ value: 'scale', name: 'Scale', label: 'Scale (R)', icon: 'maximize-2', key: 'R' }
							]}
							onchoose={(v) => (tool = v)}
						/>
						<ToolGroup
							label="Pick"
							icon="crosshair"
							selected={pick}
							options={[
								{ value: 'entities', name: 'Entities', label: 'Pick entities', icon: 'box' },
								{ value: 'targets', name: 'Targets', label: 'Pick targets', icon: 'target' }
							]}
							onchoose={(v) => (pick = pick.includes(v) ? pick.filter((p) => p !== v) : [...pick, v])}
						/>
						<ToolMenu label="Grid & snap" title={snap ? `Snap on · ${grid ?? 1} m` : 'Snap off'} icon="grid" marked={snap}>
							<div class="row"><span>Snap</span><Toggle on={snap} label="Snap" onchange={(on) => (snap = on)} /></div>
							<div class="row"><span>Grid</span><ScrubNumber label="Grid" value={grid} unit="m" step={0.25} min={0.25} onchange={(v) => (grid = v)} /></div>
						</ToolMenu>
					</div>
				{/snippet}
				{#snippet tab(id)}
					{#if id === 'outline'}
						<div class="list">
							{#each ['Chapter 3', '· Slice 1', '· Slice 2', '·· Catpope', '·· Lantern', '· Slice 3'] as line (line)}
								<ListRow dense label={line} selected={line.includes('Catpope')} onclick={() => {}} />
							{/each}
						</div>
					{:else if id === 'history'}
						<ul class="list">
							<li>Today 14:02 · draft</li>
							<li>Yesterday · published</li>
						</ul>
					{:else if id === 'inspector'}
						<div class="sheet">
							<Section id="kit-transform" label="Transform" summary="x {x ?? 0} · y {y ?? 0}">
								<div class="row"><span>X</span><ScrubNumber label="X" tag="X" value={x} unit="px" onchange={(v) => (x = v)} /></div>
								<div class="row"><span>Y</span><ScrubNumber label="Y" tag="Y" value={y} unit="px" onchange={(v) => (y = v)} /></div>
								<div class="row"><span>Scale</span><ScrubNumber label="Scale" value={scale} step={0.05} min={0.05} placeholder="1" onchange={(v) => (scale = v)} /></div>
								<div class="row"><span>Fade</span><UnitNumber label="Fade" kind="kit-fade" dimension="time" stored="s" refs={{ fps: 25, bpm: 120, perBar: 4 }} value={fade} min={0} onchange={(v) => (fade = v)} testid="kit-fade" /></div>
								<div class="row"><span>Width</span><UnitNumber label="Width" kind="kit-length" dimension="length" stored="percent" refs={{ pixels: 390, web: true }} value={slot} min={0} onchange={(v) => (slot = v)} testid="kit-length" /></div>
							</Section>
							<Section id="kit-look" label="Look" summary={visible ? 'visible' : 'hidden'}>
								<div class="row"><span>Visible</span><Toggle on={visible} label="Visible" onchange={(on) => (visible = on)} /></div>
								<div class="row"><span>Shadow</span><Toggle on={shadow} label="Shadow" onchange={(on) => (shadow = on)} /></div>
							</Section>
							<Section id="kit-glow" label="Glow" summary="strength {strength ?? 0}" enabled={glow} onenabled={(on) => (glow = on)}>
								<div class="row"><span>Strength</span><ScrubNumber label="Strength" value={strength} step={0.05} min={0} max={1} onchange={(v) => (strength = v)} /></div>
								<Section id="kit-glow-tint" label="Tint" nested open={false} summary="cyan">
									<div class="row"><span>Colour</span><span class="muted">cyan</span></div>
								</Section>
							</Section>
						</div>
					{:else if id === 'problems'}
						<p class="note warn">1 warning · Slice 3 has no lettering</p>
					{:else if id === 'library'}
						<div class="tiles">
							{#each ['Catpope', 'Lantern', 'Feilchen', 'Harbour', 'Moon', 'Choir'] as name (name)}
								<div class="tile"><span class="art"></span>{name}</div>
							{/each}
						</div>
					{:else if id === 'wanted'}
						<ul class="list">
							<li>Catpope · sad mood</li>
							<li>Harbour · night look</li>
						</ul>
					{:else}
						<p class="note">Point at a control to read what it does.</p>
					{/if}
				{/snippet}
			</FloatingPanels>
		{/key}
		{/if}
	</main>
</div>

<style>
	.kit {
		display: flex;
		flex-direction: column;
		height: 100vh;
		background: var(--tool-ground);
		color: var(--tool-text);
		font-family: var(--editor-sans);
	}
	/* the bar stands on the stage: pills, no ground of its own (RULES H11) */
	.bar {
		display: flex;
		align-items: center;
		gap: var(--kit-gap);
		padding: var(--kit-margin);
	}
	.name {
		margin-right: 0.75rem;
	}
	.spacer {
		flex: 1;
	}
	.room {
		position: relative;
		flex: 1;
		min-height: 0;
	}
	.room.scroll {
		overflow: auto;
	}
	.stage-art {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		background:
			linear-gradient(var(--tool-line) 1px, transparent 1px) 0 0 / 3rem 3rem,
			linear-gradient(90deg, var(--tool-line) 1px, transparent 1px) 0 0 / 3rem 3rem,
			var(--tool-ground);
		color: var(--tool-muted);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.tools {
		display: flex;
		align-items: center;
		gap: 0.375rem;
	}
	.sheet {
		padding-bottom: 0.5rem;
	}
	.row {
		display: grid;
		grid-template-columns: 6rem minmax(0, 1fr);
		align-items: center;
		gap: 0.5rem;
		min-height: var(--control-height);
	}
	.row > span:first-child {
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
	.list {
		display: grid;
		padding: 0.25rem;
		white-space: pre;
	}
	.note {
		margin: 0;
		padding: var(--editor-pad);
		font-family: inherit;
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
	.note.warn {
		color: var(--tool-warn);
	}
	.muted {
		color: var(--tool-muted);
	}
	.tiles {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(6rem, 1fr));
		gap: 0.5rem;
		padding: var(--editor-pad);
	}
	.tile {
		display: grid;
		gap: 0.25rem;
		font-size: var(--editor-small);
	}
	/* an empty thumbnail: its line on the stage, no placeholder ground (RULES H11) */
	.art {
		aspect-ratio: 1;
		border: var(--editor-edge) solid var(--kit-thumb-rest-border);
		border-radius: var(--editor-radius);
	}
</style>
