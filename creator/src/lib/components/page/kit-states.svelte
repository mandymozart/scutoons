<script lang="ts">
	import { Segmented } from '@scutoons/ui';
	import { Button, Chip, Field, IconButton, ListRow, MenuItem, MenuPanel, Pill, PillTab, PillTabs, Section, Select, Tile, Toggle } from '@scutoons/ui/editor';

	/**
	 * The kit's components in their states, one row each (kit/README.md, agents/research/redesign/lap2/kit-d.html): rest ·
	 * hover · on · focus · disabled · dragged · more. The real components; hover, focus and dragged are shown through
	 * `data-kit-state` (the states' colours come from kit.css, generated from kit/tokens.toml).
	 */
	const STATES = ['rest', 'hover', 'on', 'focus', 'disabled', 'dragged', 'more'] as const;
	const WORKSPACES = [
		{ id: 'scene', label: 'Scene' },
		{ id: 'map', label: 'Map' },
		{ id: 'cast', label: 'Cast' }
	];
	const SORTS = [
		{ value: 'name', label: 'By name' },
		{ value: 'use', label: 'By use' }
	];
	const LEGS = [
		{ value: 'legs', label: '2 legs' },
		{ value: 'wings', label: 'wings' }
	];
	const noop = () => {};
	let text = $state('Kathedrale');
</script>

<div class="states" data-testid="kit-states">
	<span class="head"></span>
	{#each STATES as s (s)}<span class="head">{s}</span>{/each}

	<span class="name">Pill</span>
	<span><Pill><IconButton label="Undo" icon="corner-up-left" /><IconButton label="Redo" icon="corner-up-right" /></Pill></span>
	<span></span><span></span><span></span><span></span><span></span><span class="muted">a group or a toolbar</span>

	<span class="name">PillTab</span>
	<span><PillTab label="Catpope" icon="user" /></span>
	<span><PillTab label="Catpope" icon="user" data-kit-state="hover" /></span>
	<span><PillTab label="Catpope" icon="user" on /></span>
	<span><PillTab label="Catpope" icon="user" data-kit-state="focus" /></span>
	<span><PillTab label="Catpope" icon="user" disabled /></span>
	<span><PillTab label="Catpope" icon="user" data-kit-state="dragged" /></span>
	<span><PillTab label="Kathedrale" icon="film" on onclose={noop} closeLabel="Close" /></span>

	<span class="name">Workspace row</span>
	<span class="wide"><Pill as="nav" label="Workspaces"><PillTabs items={WORKSPACES} on="scene" /></Pill></span>

	<span class="name">IconButton</span>
	<span><IconButton label="Settings" icon="settings" /></span>
	<span><IconButton label="Settings" icon="settings" data-kit-state="hover" /></span>
	<span><IconButton label="Settings" icon="settings" pressed /></span>
	<span><IconButton label="Settings" icon="settings" data-kit-state="focus" /></span>
	<span><IconButton label="Settings" icon="settings" disabled /></span>
	<span></span>
	<span class="row"><IconButton label="Search" icon="search" alone /><IconButton label="Publish" icon="upload-cloud" dot /></span>

	<span class="name">Button primary</span>
	<span><Button variant="primary">Publish</Button></span>
	<span><Button variant="primary" data-kit-state="hover">Publish</Button></span>
	<span><Button variant="primary" pressed>Publish</Button></span>
	<span><Button variant="primary" data-kit-state="focus">Publish</Button></span>
	<span><Button variant="primary" disabled>Publish</Button></span>
	<span></span>
	<span><Button variant="primary" outward>Docs</Button></span>

	<span class="name">Button plain</span>
	<span><Button>Cancel</Button></span>
	<span><Button data-kit-state="hover">Cancel</Button></span>
	<span><Button pressed>Cancel</Button></span>
	<span><Button data-kit-state="focus">Cancel</Button></span>
	<span><Button disabled>Cancel</Button></span>
	<span></span>
	<span><Button icon="folder">Open</Button></span>

	<span class="name">Button quiet</span>
	<span><Button variant="quiet">Back</Button></span>
	<span><Button variant="quiet" data-kit-state="hover">Back</Button></span>
	<span><Button variant="quiet" pressed>Back</Button></span>
	<span><Button variant="quiet" data-kit-state="focus">Back</Button></span>
	<span><Button variant="quiet" disabled>Back</Button></span>
	<span></span>
	<span><Button variant="quiet" destroy>Delete</Button></span>

	<span class="name">Segments</span>
	<span><Segmented options={LEGS} value="legs" onchange={noop} label="Legs" /></span>
	<span><span class="sc-seg"><button type="button" aria-pressed="true">2 legs</button><button type="button" data-kit-state="hover">wings</button></span></span>
	<span><Segmented options={LEGS} value="wings" onchange={noop} label="Legs" /></span>
	<span><span class="sc-seg"><button type="button" aria-pressed="true" data-kit-state="focus" class="focus">2 legs</button><button type="button">wings</button></span></span>
	<span><Segmented options={LEGS.map((o) => ({ ...o, disabled: true }))} value="legs" onchange={noop} label="Legs" /></span>
	<span></span><span></span>

	<span class="name">Chip</span>
	<span><Chip label="Rigged" onclick={noop} /></span>
	<span><Chip label="Rigged" onclick={noop} data-kit-state="hover" /></span>
	<span><Chip label="Rigged" on onclick={noop} /></span>
	<span><Chip label="Rigged" onclick={noop} data-kit-state="focus" /></span>
	<span><Chip label="Rigged" disabled onclick={noop} /></span>
	<span></span>
	<span class="row"><Chip label="hook" icon="zap" onclick={noop} /><Chip label="Deck" on onclick={noop} /></span>

	<span class="name">Status badges</span>
	<span class="wide row"><Chip label="published" tone="ok" /><Chip label="2 warnings" tone="warning" /><Chip label="1 error" tone="error" /><span class="sc-chip">draft</span><span class="sc-chip is-live">live</span><span class="sc-chip is-warning">late</span><span class="sc-chip is-error">failed</span></span>

	<span class="name">Toggle</span>
	<span><Toggle on={false} label="Snap" onchange={noop} /></span>
	<span><Toggle on={false} label="Snap" onchange={noop} data-kit-state="hover" /></span>
	<span><Toggle on label="Snap" onchange={noop} /></span>
	<span><Toggle on label="Snap" onchange={noop} data-kit-state="focus" /></span>
	<span class="row"><Toggle on={false} label="Snap" disabled onchange={noop} /><Toggle on label="Snap" disabled onchange={noop} /></span>
	<span></span>
	<span><Toggle on label="Shadow" words={['off', 'on']} onchange={noop} /></span>

	<span class="name">Field</span>
	<span><Field label="Name" bind:value={text} /></span>
	<span><Field label="Name" value="Kathedrale" data-kit-state="hover" /></span>
	<span><Field label="Name" value="Kathedrale" data-kit-state="on" /></span>
	<span><Field label="Name" value="Kathedrale" data-kit-state="focus" /></span>
	<span><Field label="Name" value="Kathedrale" disabled /></span>
	<span></span>
	<span><Field label="Name" value="" placeholder="required" aria-invalid="true" /></span>

	<span class="name">Search</span>
	<span><Field label="Find" type="search" placeholder="Find" /></span>
	<span><Field label="Find" type="search" placeholder="Find" data-kit-state="hover" /></span>
	<span></span>
	<span><Field label="Find" type="search" value="crow" data-kit-state="focus" /></span>
	<span><Field label="Find" type="search" placeholder="Find" disabled /></span>
	<span></span><span></span>

	<span class="name">Select</span>
	<span><Select label="Kind" options={SORTS} value="name" /></span>
	<span><Select label="Kind" options={SORTS} value="name" data-kit-state="hover" /></span>
	<span><Select label="Kind" options={SORTS} value="name" data-kit-state="on" /></span>
	<span><Select label="Kind" options={SORTS} value="name" data-kit-state="focus" /></span>
	<span><Select label="Kind" options={SORTS} value="name" disabled /></span>
	<span></span><span class="muted">a value in a form</span>

	<span class="name">Select quiet</span>
	<span><Select quiet label="Sort" options={SORTS} value="use" /></span>
	<span><Select quiet label="Sort" options={SORTS} value="use" data-kit-state="hover" /></span>
	<span><Select quiet label="Sort" options={SORTS} value="use" data-kit-state="on" /></span>
	<span><Select quiet label="Sort" options={SORTS} value="use" data-kit-state="focus" /></span>
	<span><Select quiet label="Sort" options={SORTS} value="use" disabled /></span>
	<span></span><span class="muted">a filter or a sort</span>

	<span class="name">Menu · MenuItem</span>
	<span class="wide">
		<MenuPanel>
			<MenuItem label="Duplicate" icon="copy" onclick={noop} />
			<MenuItem label="Rename" icon="edit-2" onclick={noop} data-kit-state="hover" />
			<MenuItem label="Night" on onclick={noop} />
			<MenuItem label="Day" on={false} onclick={noop} data-kit-state="focus" />
			<MenuItem label="Publish" icon="upload-cloud" disabled />
		</MenuPanel>
	</span>

	<span class="name">ListRow</span>
	<span><ListRow label="crow-fox" icon="image" meta="3" onclick={noop} /></span>
	<span><ListRow label="crow-fox" icon="image" meta="3" onclick={noop} data-kit-state="hover" /></span>
	<span><ListRow label="crow-fox" icon="image" meta="3" onclick={noop} selected /></span>
	<span><ListRow label="crow-fox" icon="image" meta="3" onclick={noop} data-kit-state="focus" /></span>
	<span><ListRow label="crow-fox" icon="image" meta="3" aria-disabled="true" /></span>
	<span class="stack"><ListRow label="crow-fox" icon="image" onclick={noop} data-kit-state="dragged" /><span class="insert"></span></span>
	<span><ListRow label="crow-fox" icon="image" onclick={noop} selected data-kit-state="focus" /></span>

	<span class="name">Tree row</span>
	<span><ListRow dense label="Slice 2" icon="chevron-right" onclick={noop} /></span>
	<span><ListRow dense label="Slice 2" icon="chevron-right" onclick={noop} data-kit-state="hover" /></span>
	<span><ListRow dense label="Slice 2" icon="chevron-down" onclick={noop} selected /></span>
	<span><ListRow dense label="Slice 2" icon="chevron-right" onclick={noop} data-kit-state="focus" /></span>
	<span><ListRow dense label="Slice 2" icon="chevron-right" aria-disabled="true" /></span>
	<span><ListRow dense label="Slice 2" icon="chevron-right" onclick={noop} data-kit-state="dragged" /></span>
	<span></span>

	<span class="name">Tile</span>
	<span><Tile label="Stage" icon="monitor" sub="2 outs" onclick={noop} /></span>
	<span><Tile label="Stage" icon="monitor" sub="2 outs" onclick={noop} data-kit-state="hover" /></span>
	<span><Tile label="Stage" icon="monitor" sub="2 outs" onclick={noop} on /></span>
	<span><Tile label="Stage" icon="monitor" sub="2 outs" onclick={noop} data-kit-state="focus" /></span>
	<span><Tile label="Stage" icon="monitor" sub="2 outs" disabled /></span>
	<span><Tile label="Stage" icon="monitor" sub="2 outs" onclick={noop} data-kit-state="dragged" /></span>
	<span></span>

	<span class="name">Section fold</span>
	<span class="wide row">
		<span class="sec"><Section id="kit-states-open" label="Transform" summary="x 120 · y 40"><span class="muted">rows</span></Section></span>
		<span class="sec"><Section id="kit-states-shut" label="Look" summary="visible" open={false}><span class="muted">rows</span></Section></span>
	</span>

	<span class="name">Drop · insert</span>
	<span><span class="drop">Drop here</span></span>
	<span></span>
	<span><span class="drop on">Drop here</span></span>
	<span></span><span></span>
	<span class="stack"><span class="insert"></span></span>
	<span></span>

	<span class="name">Link</span>
	<span><a class="link" href="#kit">Open in the deck</a></span>
	<span><a class="link hover" href="#kit">Open in the deck</a></span>
	<span></span>
	<span><a class="link focus" href="#kit">Open in the deck</a></span>
	<span><span class="link off">Open in the deck</span></span>
	<span></span><span></span>

	<span class="name">Status</span>
	<span class="wide row">
		{#each ['none', 'ok', 'info', 'warning', 'error'] as tone (tone)}<span class="status"><i class="dot {tone}"></i>{tone}</span>{/each}
	</span>
</div>

<style>
	.states {
		display: grid;
		grid-template-columns: 8rem repeat(7, minmax(9rem, max-content));
		align-items: center;
		gap: 0.75rem 1rem;
		padding: 1.5rem 2rem 3rem;
		color: var(--tool-text);
	}
	.head {
		color: var(--tool-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.name {
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
	.muted {
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
	.wide {
		grid-column: span 7;
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem;
	}
	.stack {
		display: grid;
		gap: 0.25rem;
	}
	.sec {
		display: block;
		width: 17rem;
	}
	.focus {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: 1px;
	}
	.drop {
		display: grid;
		place-items: center;
		width: 9rem;
		height: 3rem;
		border: var(--editor-edge) dashed var(--kit-drop-rest-border);
		border-radius: var(--kit-radius-box);
		color: var(--kit-drop-rest-text);
		font-size: var(--editor-small);
	}
	.drop.on {
		border-color: var(--kit-drop-on-border);
		background: var(--kit-drop-on-ground);
		color: var(--kit-drop-on-text);
	}
	.insert {
		display: block;
		height: 2px;
		border-radius: 1px;
		background: var(--kit-list-row-insert-mark);
	}
	.link {
		color: var(--kit-link-rest-text);
		text-decoration: underline;
		text-decoration-thickness: 1px;
		text-underline-offset: 0.2em;
	}
	.link.hover {
		text-decoration-thickness: 2px;
	}
	.link.off {
		color: var(--kit-link-disabled-text);
		text-decoration: none;
	}
	.status {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
	.dot {
		width: 0.5rem;
		height: 0.5rem;
		border-radius: 50%;
	}
	.dot.none {
		box-shadow: inset 0 0 0 var(--editor-edge) var(--kit-status-none-mark);
	}
	.dot.ok {
		background: var(--kit-status-ok-mark);
	}
	.dot.info {
		background: var(--kit-status-info-mark);
	}
	.dot.warning {
		background: var(--kit-status-warning-mark);
	}
	.dot.error {
		background: var(--kit-status-error-mark);
	}
</style>
