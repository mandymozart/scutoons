<script lang="ts">
	import { ScrubNumber } from '@scutoons/ui';
	import { readSettings, SETTINGS_KEY } from '$lib/shell/settings.ts';
	import { at, Floats, FloatingPanels, Section, Toggle, ToolGroup, ToolMenu, type PanelSpec, type TabSpec } from '@scutoons/ui/editor';

	// Dev only: the editor kit over a stage – drag a panel by its grip, a tab into another panel or out onto the stage,
	// resize an edge (the neighbours make room), fold by a double-click on the head; reload and the layout is back.
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
	const WORKSPACES = ['Scene', 'Character'] as const;

	let workspace = $state<(typeof WORKSPACES)[number]>('Scene');
	const floats = $derived(new Floats(`kit-${workspace.toLowerCase()}`, PANELS, TABS));
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
	let visible = $state(true);
	let shadow = $state(false);
	let glow = $state(true);
	let strength = $state<number | undefined>(0.35);
</script>

<svelte:head><title>Editor kit</title></svelte:head>

<div class="kit sc-tool" data-theme={day ? 'day' : 'night'}>
	<header class="bar">
		<span class="name">Editor kit</span>
		{#each WORKSPACES as w (w)}
			<button type="button" class="pill" class:on={w === workspace} onclick={() => (workspace = w)}>{w}</button>
		{/each}
		<span class="spacer"></span>
		<button type="button" class="pill" onclick={() => floats.reset()}>Reset layout</button>
	</header>
	<main class="room">
		{#key floats}
			<FloatingPanels {floats}>
				<div class="stage-art"><span>Stage · {workspace}</span></div>
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
						<ul class="list">
							{#each ['Chapter 3', '· Slice 1', '· Slice 2', '·· Catpope', '·· Lantern', '· Slice 3'] as line (line)}
								<li class:sel={line.includes('Catpope')}>{line}</li>
							{/each}
						</ul>
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
	</main>
</div>

<style>
	.kit {
		display: flex;
		flex-direction: column;
		height: 100vh;
		background: var(--editor-stage);
		color: var(--editor-text);
		font-family: var(--editor-sans);
	}
	.bar {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.5rem 1rem;
		background: var(--editor-surface-head);
		border-bottom: var(--editor-edge) solid var(--editor-line);
	}
	.name {
		margin-right: 0.75rem;
	}
	.spacer {
		flex: 1;
	}
	.pill {
		height: var(--control-height);
		padding: 0 0.875rem;
		border: 0;
		border-radius: 1.2rem;
		background: var(--editor-control);
		color: var(--editor-muted);
		font: inherit;
		cursor: pointer;
	}
	.pill.on {
		background: var(--editor-pressed);
		color: var(--editor-pressed-text);
	}
	.room {
		flex: 1;
		min-height: 0;
	}
	.stage-art {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		background:
			linear-gradient(var(--editor-line) 1px, transparent 1px) 0 0 / 3rem 3rem,
			linear-gradient(90deg, var(--editor-line) 1px, transparent 1px) 0 0 / 3rem 3rem,
			radial-gradient(circle at 50% 45%, var(--editor-within), var(--editor-stage) 70%);
		color: var(--editor-muted);
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
		color: var(--editor-muted);
		font-size: var(--editor-small);
	}
	.list {
		margin: 0;
		padding: 0.25rem 0;
		list-style: none;
	}
	.list li {
		display: flex;
		align-items: center;
		height: var(--editor-tree-row);
		padding: 0 var(--editor-pad);
		white-space: pre;
	}
	.list li.sel {
		background: var(--editor-selected);
	}
	.note {
		margin: 0;
		padding: var(--editor-pad);
		font-family: inherit;
		color: var(--editor-muted);
		font-size: var(--editor-small);
	}
	.note.warn {
		color: var(--editor-warn);
	}
	.muted {
		color: var(--editor-muted);
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
	.art {
		aspect-ratio: 1;
		border-radius: var(--editor-radius);
		background: linear-gradient(135deg, var(--editor-control), var(--editor-selected));
	}
</style>
