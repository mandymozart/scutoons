<script lang="ts">
	import { CloseButton, Icon } from '@scutoons/ui';
	import type { AssetFamily, LibraryScope } from '@scutoons/media';
	import { getCreatorNav } from '$lib/nav.svelte.ts';
	import { library } from '$lib/editor/library.svelte.ts';
	import { deleteFamilies, downloadFamilies, renameFamily } from '$lib/editor/library-actions.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { matches, places, type Place } from '$lib/shell/places.svelte.ts';
	import AssetDetails from '$lib/components/library/asset-details.svelte';
	import LibraryGrid from '$lib/components/library/library-grid.svelte';
	import LibraryMenu from '$lib/components/library/library-menu.svelte';
	import LibraryOpenings from '$lib/components/library/library-openings.svelte';
	import { openings } from '$lib/shell/openings.svelte.ts';
	import LibraryToolbar from '$lib/components/library/library-toolbar.svelte';

	/**
	 * The Library panel (research/redesign/library.html ?w=panel, OSCT's LibraryPanel): the scope – what the open item
	 * uses, the toon, All (the open item pinned, the rest folded), Unused –, search (Ctrl L), Upload or files dropped
	 * anywhere on it, the kinds the core counted, the view tools, the groups; a click selects (Ctrl / Shift several) and
	 * opens the details at the bottom; F2 renames, Delete deletes, right-click or ⋯ has every action. A blank slot's
	 * pick puts the slot on top and shows only what fits. "Items to open" lists what opens as a tab.
	 */
	const nav = getCreatorNav();
	$effect(() => places.load(nav.toons));
	$effect(() => void library.load());

	let root = $state<HTMLElement>();
	let search: HTMLInputElement | null = $state(null);
	let fileInput: HTMLInputElement | null = $state(null);
	let replaceInput: HTMLInputElement | null = $state(null);
	let replacing: AssetFamily | null = null;
	let note = $state('');
	let dropping = $state(false);
	let menu = $state<{ x: number; y: number } | null>(null);

	const view = $derived(library.view);
	const kindOfOpen = $derived.by(() => {
		const k = library.address?.split(':')[0] ?? '';
		return k === 'storyboard' ? 'chapter' : ['chapter', 'season', 'character', 'characters', 'map', 'toon'].includes(k) ? k : k ? 'items' : '';
	});
	const scopes = $derived<{ id: LibraryScope; label: string; count: number }[]>([
		...(kindOfOpen && kindOfOpen !== 'toon' ? [{ id: 'context' as const, label: m[`library.scope_${kindOfOpen as 'chapter'}`](), count: view?.scopes.context ?? 0 }] : []),
		...(library.toon ? [{ id: 'toon' as const, label: m['library.scope_toon'](), count: view?.scopes.toon ?? 0 }] : []),
		{ id: 'all', label: m['library.scope_all'](), count: view?.scopes.all ?? 0 },
		{ id: 'unused', label: m['library.scope_unused'](), count: view?.scopes.unused ?? 0 }
	]);
	$effect(() => {
		if (!scopes.some((s) => s.id === library.scope)) library.setScope(scopes[0].id);
	});
	const entities = $derived.by<Place[]>(() => {
		const all = [...places.toons(nav.toons, (id) => nav.toonName(id), library.toon), ...places.areas(), ...places.experiences()];
		const here = library.scope !== 'all' && library.toon ? all.filter((p) => p.toon === library.toon) : all;
		return here.filter((p) => matches(p, library.query));
	});
	const openable = $derived(openings.groups(library.query, library.scope !== 'all' ? library.toon : null).reduce((n, g) => n + g.rows.length, 0));

	const typing = (e: KeyboardEvent) => e.target instanceof HTMLElement && !!e.target.closest('input, textarea, select, [contenteditable]');

	function keys(event: KeyboardEvent) {
		if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'l') {
			event.preventDefault();
			search?.focus();
			search?.select();
		} else if (event.key === 'Escape' && !event.defaultPrevented && library.pick && !library.browser) library.cancelPick();
		else if (!root?.contains(document.activeElement) || typing(event) || library.browser) return;
		else if (event.key === 'F2' && library.selectedFamily) (event.preventDefault(), renameFamily(library.selectedFamily));
		else if (event.key === 'Delete' && library.chosen.length) (event.preventDefault(), deleteFamilies(library.chosenFamilies));
	}

	async function upload(files: File[]) {
		note = '';
		for (const file of files) {
			try {
				const { converted } = await library.upload(file);
				if (converted) note = m['library.converted']({ name: file.name, path: converted });
			} catch (e) {
				note = m['library.upload_failed']({ reason: e instanceof Error ? e.message : String(e) });
			}
		}
	}

	async function replace(file: File) {
		if (!replacing) return;
		note = '';
		try {
			await library.replace(replacing, file);
		} catch (e) {
			note = m['library.upload_failed']({ reason: e instanceof Error ? e.message : String(e) });
		}
	}

	const hasFiles = (e: DragEvent) => !!e.dataTransfer?.types.includes('Files');
</script>

<svelte:window onkeydown={keys} />

<div
	bind:this={root}
	class="library"
	class:picking={!!library.pick}
	class:dropping
	data-testid="library"
	role="region"
	aria-label={m['library.title']()}
	ondragover={(e) => {
		if (!hasFiles(e)) return;
		e.preventDefault();
		dropping = true;
	}}
	ondragleave={(e) => {
		if (!root?.contains(e.relatedTarget as Node)) dropping = false;
	}}
	ondrop={(e) => {
		if (!hasFiles(e)) return;
		e.preventDefault();
		dropping = false;
		upload([...(e.dataTransfer?.files ?? [])]);
	}}
>
	<div class="head">
		{#if library.pick}
			<div class="pick" data-testid="library-pick">
				<span>{m['library.pick_for']()} <span class="slot">{library.pick.label}</span></span>
				<button type="button" class="link" data-control-exempt="text link" onclick={() => library.cancelPick()}>{m['library.cancel_esc']()}</button>
			</div>
		{/if}
		<div class="row">
			<div class="chips" role="radiogroup" aria-label={m['library.scope']()}>
				{#each scopes as s (s.id)}
					<button type="button" class="chip" data-control-exempt="filter chip" role="radio" aria-checked={library.scope === s.id} onclick={() => library.setScope(s.id)} data-testid="scope-{s.id}">{s.label}<span class="n">{s.count}</span></button>
				{/each}
			</div>
		</div>
		<div class="row">
			<input bind:this={search} class="find" type="search" bind:value={library.query} placeholder={m['library.search']()} aria-label={m['library.search']()} data-testid="library-search" />
			<button type="button" class="primary" title={m['library.upload_title']()} onclick={() => fileInput?.click()} data-testid="library-upload"><Icon name="upload" size="0.9rem" /> {m['library.upload']()}</button>
			<input bind:this={fileInput} type="file" multiple hidden onchange={(e) => (upload([...(e.currentTarget.files ?? [])]), (e.currentTarget.value = ''))} />
			<input bind:this={replaceInput} type="file" hidden onchange={(e) => { const f = e.currentTarget.files?.[0]; if (f) replace(f); e.currentTarget.value = ''; }} />
		</div>
		<div class="chips">
			<button type="button" class="chip" data-control-exempt="filter chip" aria-pressed={!library.kind && !library.openings} onclick={() => ((library.kind = null), (library.openings = false))}>{m['library.kind_all']()}</button>
			{#each view?.kinds ?? [] as [k, n] (k)}
				<button type="button" class="chip" data-control-exempt="filter chip" aria-pressed={library.kind === k && !library.openings} onclick={() => ((library.kind = k), (library.openings = false))} data-testid="kind-{k}">{m[`library.kinds_${k}`]()}<span class="n">{n}</span></button>
			{/each}
			{#if !library.pick}
				<button type="button" class="chip" data-control-exempt="filter chip" aria-pressed={library.openings} onclick={() => (library.openings ? (library.openings = false) : library.showOpenings(null))} data-testid="kind-entities">{m['library.entities']()}<span class="n">{entities.length + openable}</span></button>
			{/if}
		</div>
		<LibraryToolbar />
		{#if library.chosen.length > 1}
			<div class="selection" data-testid="library-selection">
				<span>{m['library.n_selected']({ count: library.chosen.length })}</span>
				<button type="button" class="quiet" onclick={() => downloadFamilies(library.chosenFamilies)}><Icon name="download" size="0.85rem" />{m['library.download']()}</button>
				<button type="button" class="quiet danger" onclick={() => deleteFamilies(library.chosenFamilies)}><Icon name="trash-2" size="0.85rem" />{m['library.delete']()}</button>
				<CloseButton label={m['library.clear_selection']()} onclick={() => library.clearSelection()} />
			</div>
		{/if}
		{#if note}<p class="sc-empty note">{note}</p>{/if}
	</div>

	<div class="body">
		{#if library.openings}
			<LibraryOpenings places={library.openKind ? [] : entities} only={library.openKind} query={library.query} toon={library.scope !== 'all' ? library.toon : null} onkind={(k) => ((library.kind = k), (library.openings = false))} onnote={(text) => (note = text)} />
		{:else if library.status === 'error'}
			<p class="sc-empty">{m['library.load_failed']({ reason: library.error })}</p>
		{:else if library.status === 'ready' && !view?.total}
			<p class="sc-empty">{library.query ? m['library.none_found']() : m['library.none_here']()}</p>
		{:else}
			<LibraryGrid tile={library.tile} list={library.list} onmenu={(x, y) => (menu = { x, y })} onuse={(f) => library.use(f)} />
		{/if}
	</div>

	{#if library.details && library.selectedFamily && library.chosen.length === 1 && !library.browser}
		<AssetDetails family={library.selectedFamily} drawer onreplace={(file) => ((replacing = library.selectedFamily), replace(file))} onclose={() => (library.details = false)} />
	{/if}
	{#if dropping}<div class="drop" aria-hidden="true">{m['library.drop_here']()}</div>{/if}
</div>

{#if menu}
	<LibraryMenu x={menu.x} y={menu.y} onclose={() => (menu = null)} onreplace={(f) => ((replacing = f), replaceInput?.click())} />
{/if}

<style>
	.library {
		position: relative;
		display: flex;
		flex-direction: column;
		min-height: 100%;
		padding: 0 var(--editor-pad) 0;
	}
	.head {
		position: sticky;
		z-index: 1;
		top: 0;
		display: flex;
		flex-direction: column;
		gap: 0.45rem;
		padding-bottom: 0.45rem;
		background: transparent;
	}
	.body {
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		padding-bottom: var(--editor-pad);
	}
	.pick,
	.selection {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.3rem 0.5rem;
		border: var(--editor-edge) solid var(--tool-pressed);
		border-radius: var(--editor-radius);
		font-size: var(--editor-small);
	}
	.pick > span,
	.selection > span {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.slot {
		color: var(--tool-text);
	}
	.link {
		border: 0;
		background: none;
		color: var(--tool-text);
		cursor: pointer;
		font: inherit;
	}
	.quiet {
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
		border: 0;
		background: none;
		color: var(--tool-text);
		cursor: pointer;
		font: inherit;
	}
	.quiet:disabled {
		color: var(--tool-muted);
		cursor: default;
	}
	.danger {
		color: var(--tool-danger);
	}
	.row {
		display: flex;
		align-items: center;
		gap: 0.35rem;
	}
	.row .chips {
		flex: 1 1 auto;
	}
	.find {
		flex: 1 1 auto;
		min-width: 0;
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
	.chip {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		min-height: 1.7rem;
		padding: 0 0.6rem;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: 1rem;
		background: var(--tool-control);
		color: var(--tool-text);
		font-size: var(--editor-small);
		cursor: pointer;
	}
	.chip[aria-pressed='true'],
	.chip[aria-checked='true'] {
		background: var(--tool-pressed);
		color: var(--tool-pressed-text);
	}
	.n {
		font-size: var(--editor-label);
		opacity: 0.7;
	}
	.primary {
		flex: none;
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		padding: 0 0.7rem;
		border: var(--editor-edge) solid var(--tool-pressed);
		border-radius: var(--editor-radius);
		background: var(--tool-pressed);
		color: var(--tool-pressed-text);
		cursor: pointer;
	}
	.note {
		margin: 0;
		color: var(--tool-warn);
	}
	.drop {
		position: absolute;
		inset: 0;
		z-index: 2;
		display: grid;
		place-items: center;
		border: var(--editor-edge) dashed var(--tool-text);
		border-radius: var(--editor-radius);
		background: color-mix(in srgb, var(--tool-popover) 80%, transparent);
		color: var(--tool-text);
		pointer-events: none;
	}
</style>
