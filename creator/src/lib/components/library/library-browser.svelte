<script lang="ts">
	import { CloseButton, Icon } from '@scutoons/ui';
	import type { AssetFamily } from '@scutoons/media';
	import { library } from '$lib/editor/library.svelte.ts';
	import { deleteFamilies, renameFamily } from '$lib/editor/library-actions.ts';
	import { m } from '$lib/paraglide/messages.js';
	import AssetDetails from '$lib/components/library/asset-details.svelte';
	import LibraryGrid from '$lib/components/library/library-grid.svelte';
	import LibraryMenu from '$lib/components/library/library-menu.svelte';
	import LibraryPlaces from '$lib/components/library/library-places.svelte';
	import LibraryToolbar from '$lib/components/library/library-toolbar.svelte';

	/**
	 * The Library over the stage (⤢, research/redesign/library.html ?w=browser): the places at the left, the core's
	 * groups in the middle, the details at the right; the levels stacked or single. Esc closes it back to the panel.
	 */
	let fileInput: HTMLInputElement | null = $state(null);
	let replaceInput: HTMLInputElement | null = $state(null);
	let replacing: AssetFamily | null = null;
	let note = $state('');
	let menu = $state<{ x: number; y: number } | null>(null);
	const view = $derived(library.view);

	function keys(event: KeyboardEvent) {
		const typing = event.target instanceof HTMLElement && !!event.target.closest('input, textarea, select');
		if (event.key === 'Escape' && !menu) {
			event.preventDefault();
			library.browser = false;
		} else if (typing) return;
		else if (event.key === 'F2' && library.selectedFamily) (event.preventDefault(), renameFamily(library.selectedFamily));
		else if (event.key === 'Delete' && library.chosen.length) (event.preventDefault(), deleteFamilies(library.chosenFamilies));
	}

	async function run(work: () => Promise<unknown>) {
		note = '';
		try {
			await work();
		} catch (e) {
			note = m['library.upload_failed']({ reason: e instanceof Error ? e.message : String(e) });
		}
	}
	const upload = (files: File[]) =>
		run(async () => {
			for (const f of files) {
				const { converted } = await library.upload(f);
				if (converted) note = m['library.converted']({ name: f.name, path: converted });
			}
		});
</script>

<svelte:window onkeydown={keys} />

<div class="browser" role="dialog" aria-label={m['library.title']()} data-testid="library-browser">
	<header>
		<span class="lbl">{m['library.title']()}</span>
		<input class="find" type="search" bind:value={library.query} placeholder={m['library.search']()} aria-label={m['library.search']()} />
		<div class="chips">
			<button type="button" class="chip" data-control-exempt="filter chip" aria-pressed={!library.kind} onclick={() => (library.kind = null)}>{m['library.kind_all']()}</button>
			{#each view?.kinds ?? [] as [k, n] (k)}
				<button type="button" class="chip" data-control-exempt="filter chip" aria-pressed={library.kind === k} onclick={() => (library.kind = k)}>{m[`library.kinds_${k}`]()}<span class="n">{n}</span></button>
			{/each}
		</div>
		<span class="sp"></span>
		<div class="tools"><LibraryToolbar levels /></div>
		<button type="button" class="primary" onclick={() => fileInput?.click()}><Icon name="upload" size="0.9rem" /> {m['library.upload']()}</button>
		<input bind:this={fileInput} type="file" multiple hidden onchange={(e) => (upload([...(e.currentTarget.files ?? [])]), (e.currentTarget.value = ''))} />
		<input bind:this={replaceInput} type="file" hidden onchange={(e) => { const f = e.currentTarget.files?.[0]; if (f && replacing) run(() => library.replace(replacing!, f)); e.currentTarget.value = ''; }} />
		<CloseButton label={m['library.close_browser']()} onclick={() => (library.browser = false)} />
	</header>
	<div class="body" class:with-details={!!library.selectedFamily && library.chosen.length === 1}>
		<LibraryPlaces />
		<div class="main">
			{#if note}<p class="sc-empty note">{note}</p>{/if}
			<LibraryGrid tile={Math.max(library.tile, 8)} list={library.list} onmenu={(x, y) => (menu = { x, y })} onuse={(f) => (library.use(f), (library.browser = false))} />
			{#if library.status === 'ready' && !view?.total}<p class="sc-empty">{m['library.none_found']()}</p>{/if}
			{#if library.status === 'loading'}<p class="sc-empty">{m['library.loading']()}</p>{/if}
			<p class="foot">{m['library.browser_foot']({ count: view?.total ?? 0 })}</p>
		</div>
		{#if library.selectedFamily && library.chosen.length === 1}
			<AssetDetails family={library.selectedFamily} onreplace={(file) => run(() => library.replace(library.selectedFamily!, file))} />
		{/if}
	</div>
</div>

{#if menu}
	<LibraryMenu x={menu.x} y={menu.y} onclose={() => (menu = null)} onreplace={(f) => ((replacing = f), replaceInput?.click())} />
{/if}

<style>
	.browser {
		position: fixed;
		z-index: 80; /* over everything, the bar too (as OSCT's asset manager) */
		top: 2rem;
		left: 2.5rem;
		right: 2.5rem;
		bottom: 2.5rem;
		box-shadow: 0 0 0 100vmax var(--tool-scrim);
		display: flex;
		flex-direction: column;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: var(--editor-radius);
		background: var(--tool-popover);
		color: var(--tool-text);
		pointer-events: auto;
		overflow: hidden;
	}
	header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.45rem var(--editor-pad);
		border-bottom: var(--editor-edge) solid var(--tool-line);
	}
	.lbl {
		color: var(--tool-muted);
		font-size: var(--editor-label);
		letter-spacing: var(--editor-label-spacing, 0.08em);
		text-transform: uppercase;
	}
	.find {
		width: 22rem;
		max-width: 30%;
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
	.chip[aria-pressed='true'] {
		background: var(--tool-pressed);
		color: var(--tool-pressed-text);
	}
	.n {
		font-size: var(--editor-label);
		opacity: 0.7;
	}
	.sp {
		flex: 1 1 auto;
	}
	.tools {
		min-width: 24rem;
	}
	.primary {
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
	.body {
		flex: 1 1 auto;
		min-height: 0;
		display: grid;
		grid-template-columns: 15rem minmax(0, 1fr);
	}
	.body.with-details {
		grid-template-columns: 15rem minmax(0, 1fr) 24rem;
	}
	.main {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		overflow: auto;
		padding: 0.5rem var(--editor-pad) var(--editor-pad);
	}
	.foot {
		margin: 0.8rem 0 0;
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
	.note {
		color: var(--tool-warn);
	}
</style>
