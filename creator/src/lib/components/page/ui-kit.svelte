<script lang="ts">
	import { Button, Card, Chip, Field, IconButton, ListRow, MenuHead, MenuItem, MenuPanel, Pill, PillTab, PillTabs, Tile, Toggle } from '@scutoons/ui/editor';

	/**
	 * The UI kit on one sheet (RULES H12; /guide#ui-kit): every component of @scutoons/ui/editor by night and by day, side
	 * by side on the stage, to hold against the deck editor (deck-kit's controls.rs, bar.rs, floats.rs, fold.rs, home.rs).
	 * The floating panel's head and tabs are live on every workspace and on the dev page /kit.
	 */
	const THEMES = ['night', 'day'] as const;
	let on = $state(true);
	let tab = $state('scene');
	let mode = $state('edit');
	let text = $state('Kathedrale');
	const WORKSPACES = [
		{ id: 'toon', label: 'Toon' },
		{ id: 'chapter', label: 'Chapter' },
		{ id: 'scene', label: 'Scene' },
		{ id: 'map', label: 'Map' }
	];
</script>

<div class="kit" data-testid="ui-kit">
	{#each THEMES as theme (theme)}
		<div class="sheet" data-theme={theme} data-control-exempt="the kit's sheet shows the bar's inset sizes" data-testid="ui-kit-{theme}">
			<h3>{theme === 'night' ? 'Night' : 'Day'}</h3>

			<p class="cap">Bar · Pill, IconButton, PillTab (concentric; the active tab in the text colour, no border at rest; groups and toolbars bordered, a lone icon round)</p>
			<div class="bar">
				<Pill><IconButton label="Home" icon="home" pressed /></Pill>
				<Pill>
					<PillTab label="Kathedrale" icon="film" href="#ui-kit" on closeLabel="Close" onclose={() => {}} />
					<PillTab label="Catpope" icon="user" href="#ui-kit" closeLabel="Close" onclose={() => {}} />
					<IconButton label="Open" icon="plus" />
				</Pill>
			</div>
			<div class="bar">
				<Pill as="nav" label="Workspaces"><PillTabs items={WORKSPACES} on={tab} onselect={(id) => (tab = id)} /></Pill>
				<Pill>
					<IconButton label="Publish" icon="upload-cloud" dot />
					<IconButton label="Account" icon="user" />
					<IconButton label="Menu" icon="menu" aria-expanded="true" />
				</Pill>
			</div>

			<p class="cap">Button</p>
			<div class="row">
				<Button variant="primary" icon="upload-cloud">Publish</Button>
				<Button variant="primary" outward>Docs</Button>
				<Button icon="folder">Open</Button>
				<Button variant="quiet" icon="copy">Copy</Button>
				<Button pressed>Pressed</Button>
				<Button disabled>Off</Button>
			</div>

			<p class="cap">IconButton · PillTabs · Chip · Toggle</p>
			<div class="row">
				<IconButton label="Settings" icon="settings" />
				<IconButton label="Pinned" icon="map-pin" pressed />
				<IconButton label="Play" icon="play" alone />
				<IconButton label="Stop" icon="square" tone="stop" />
				<PillTabs small items={[{ id: 'edit', label: 'Edit' }, { id: 'play', label: 'Play' }]} on={mode} onselect={(id) => (mode = id)} />
			</div>
			<div class="row">
				<Chip label="guest" on />
				<Chip label="member" />
				<Chip label="sound" icon="music" />
				<Toggle label="Snap" {on} onchange={(v) => (on = v)} />
				<Toggle label="Grid" on={!on} onchange={(v) => (on = !v)} />
			</div>

			<p class="cap">Field</p>
			<Field label="Name" bind:value={text} />
			<Field label="Toon" value="catpope" disabled />

			<p class="cap">Menu · MenuPanel, MenuItem, MenuHead</p>
			<MenuPanel width="17rem" role="presentation">
				<MenuItem icon="home" label="Home" current />
				<hr />
				<MenuItem icon="folder" label="Open" kbd="Ctrl O" onclick={() => {}} />
				<MenuItem icon="layout" label="Views" sub expanded onclick={() => {}} />
				<MenuItem icon="book" label="Docs" href="#ui-kit" external />
				<MenuItem label="Close others" disabled />
				<MenuHead label="Panels" />
				<MenuItem label="Info" on={false} onclick={() => {}} />
				<MenuItem label="Mixer" on onclick={() => {}} />
			</MenuPanel>

			<p class="cap">Card · ListRow · Tile</p>
			<Card title="Not live yet" count={2}>
				<ListRow icon="film" label="Kathedrale" detail="scene" meta="today" href="#ui-kit" />
				<ListRow icon="user" label="Catpope" detail="character" meta="yesterday" href="#ui-kit" selected />
				<ListRow icon="list" label="A tree row" dense onclick={() => {}} />
			</Card>
			<div class="tiles">
				<Tile icon="film" label="Scenes" href="#ui-kit" />
				<Tile icon="map" label="Map" sub="toon:catpope" href="#ui-kit" on />
			</div>

			<p class="cap">Status bar · Pill status</p>
			<Pill status grow>
				<Button variant="quiet" fill>Saved Kathedrale</Button>
				<Button variant="quiet">0 errors</Button>
			</Pill>
		</div>
	{/each}
</div>

<style>
	.kit {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: var(--editor-gap);
	}
	/* each look on its own stage (black at night, white by day), as the creator's */
	.sheet {
		display: flex;
		flex-direction: column;
		gap: calc(var(--editor-pad) / 2);
		min-width: 0;
		padding: var(--editor-gap);
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: var(--kit-radius-box);
		background: var(--tool-ground);
		color: var(--tool-text);
	}
	h3 {
		margin: 0;
		font-size: var(--editor-title);
		font-weight: inherit;
	}
	.cap {
		margin: var(--editor-pad) 0 0;
		color: var(--tool-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.bar,
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--kit-gap);
	}
	.tiles {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: var(--kit-gap);
	}
</style>
