<script lang="ts">
	import { onMount } from 'svelte';
	import { NEUTRAL_LOCALE, type CollagePiece, type Path } from '@scutoons/content';
	import { ScoreView } from '@scutoons/reader/score';
	import { findBar, type BarRef, type PageGeometry } from '@scutoons/reader/score-anchors';
	import { Divider, Segmented, liveRegion, panelWidth, region } from '@scutoons/ui';
	import MetaPage from '$lib/components/page/meta-page.svelte';
	import ScoreFrame from '$lib/components/collage/score-frame.svelte';
	import ScoreOverlay from '$lib/components/collage/score-overlay.svelte';
	import CollageList from '$lib/components/collage/collage-list.svelte';
	import PieceInspector from '$lib/components/collage/piece-inspector.svelte';
	import CollagePickerSheet from '$lib/components/collage/collage-picker-sheet.svelte';
	import { authApi } from '$lib/api.ts';
	import { catalogKey, loadCatalogDoc } from '$lib/editor/catalog-docs.ts';
	import { loadToons, moodsDoc, moodsOf, registryDoc, registryOf, type ToonInfo } from '$lib/editor/characters.ts';
	import { pieceAt } from '$lib/editor/collage-edit.ts';
	import type { PickedPiece } from '$lib/editor/collage-pick.ts';
	import { pieceLabel, scoreViewLabels } from '$lib/editor/collage-words.ts';
	import { focusIn, isField } from '$lib/editor/editor-keys.ts';
	import { neutralDoc } from '$lib/editor/languages.ts';
	import { screenZoom, storeScreen, storedScreen, type Screen } from '$lib/editor/screens.ts';
	import * as sc from '$lib/editor/score-collage.ts';
	import { contentName } from '$lib/editor/text-language.svelte.ts';
	import { usePageNav } from '$lib/nav.svelte.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { toast } from '$lib/toasts.ts';
	import { asset } from '$lib/utils/cdn.ts';

	/**
	 * A score item's collage stage (research/COLLAGE.md §4.4): the readers' part of the score in the reader's own score
	 * view, live – on a phone or a desktop screen (ScoreFrame), its lines and pages breaking as they do for readers – with
	 * the pieces' boxes over it (ScoreOverlay: move = re-anchor to the bar and staff under it, the corner scales). Beside
	 * it (below on narrower screens) the pieces (CollageList), "Add a piece …" (the slices' picker; it lands in the middle
	 * of the page shown) and the slices' piece inspector. Edited in the item's document: Save draft, History, Publish –
	 * MetaPage. The file is the published one (signed like a reader's); the collage remembers it (`file`).
	 */
	let { id }: { id: string } = $props();

	const N = NEUTRAL_LOCALE;
	// svelte-ignore state_referenced_locally
	const doc = neutralDoc(catalogKey('items', id));
	const registry = registryDoc();
	const moods = moodsDoc();
	let toons = $state<ToonInfo[]>([]);
	onMount(() => {
		void Promise.all([loadCatalogDoc(doc, 'items', id), registry.load(), moods.load()]);
		void loadToons(getLocale()).then((list) => (toons = list));
	});

	const item = $derived(doc.loading ? null : ((doc.data[N] as Record<string, any> | null) ?? null));
	const part = $derived(sc.stagePart(item));
	const file = $derived(sc.partFile(item, part));
	const name = $derived(item ? contentName(item.name, id) || id : id);
	const pieces = $derived(sc.sheetPieces(item, part));

	// the published file, signed like a reader's (a draft's new file opens once it is published)
	let src = $state<string | null>(null);
	let unsigned = $state(false);
	$effect(() => {
		if (!file) return;
		const which = part;
		authApi
			.scoreFile(id, 'musicxml', which)
			.then((answer) => (src = answer.url))
			.catch(() => (unsigned = true));
	});

	// ── the screen: a phone or a desktop, each starting at the zoom a reader's starts with ──
	const SCREEN_KEY = 'sc-score-stage-screen';
	const firstScreen = storedScreen(SCREEN_KEY);
	let screen = $state<Screen>(firstScreen);
	let zoom = $state(screenZoom(firstScreen));
	let page = $state(1);
	function pickScreen(next: Screen) {
		if (next === screen) return;
		screen = next;
		zoom = screenZoom(next);
		storeScreen(SCREEN_KEY, next);
	}

	// ── the layout: the file's bars, each rendered page's geometry (the overlay reports it) ──
	let bars = $state.raw<BarRef[] | null>(null);
	/** page → its geometry, as last rendered (cleared by every layout) */
	const geometries = new Map<number, PageGeometry>();
	function onlayout(next: BarRef[]) {
		bars = next;
		geometries.clear();
	}
	/** the rendered page that holds the piece's bar */
	const pageOf = (piece: CollagePiece | undefined): PageGeometry | null => [...geometries.values()].find((g) => findBar(g.bars, piece?.at)) ?? null;
	const missing = $derived(sc.missingBars(pieces, bars));

	// ── selection: by id (an undo, a reorder or a removal renumber the list) ──
	let selected = $state<string | null>(null);
	const index = $derived(pieces.findIndex((p) => p.id === selected));
	const piece = $derived(index >= 0 ? pieces[index] : undefined);
	const piecePath = $derived<Path>([...sc.collagePath(part), 'pieces', index]);
	/** the selected piece's bar: the view turns to it and keeps it in view across a new layout */
	const follow = $derived(piece?.at ? { ...piece.at } : null);

	// ── edits (each one undo step; a drag's moves share their gesture's key) ──
	const edit = (change: (item: Record<string, any>) => void, key?: string) => doc.edit(change, key);
	const byId = (file: unknown, pieceId: string) => sc.sheetPieces(file, part).find((p) => p.id === pieceId);
	let picking = $state(false);
	function add(picked: PickedPiece) {
		const geo = geometries.get(page);
		const place = geo && sc.centrePlace(geo);
		if (!place) return void toast(m['creator.collage_stage_no_page'](), { type: 'warning' });
		const next = sc.newScorePiece(picked, place, sc.NEW_SCORE_PIECE_WIDTH, new Set(pieces.map((p) => p.id)));
		edit((file) => void sc.addScorePiece(file, part, next));
		selected = next.id;
	}
	const move = (pieceId: string, place: sc.ScorePlaceAt, drag: number) =>
		edit((file) => {
			const p = byId(file, pieceId);
			if (p) sc.movePiece(p, place);
		}, `stage-move-${drag}`);
	const scale = (pieceId: string, width: number, drag: number) =>
		edit((file) => {
			const p = byId(file, pieceId);
			if (p) p.rect = { ...p.rect, width };
		}, `stage-scale-${drag}`);
	function remove(pieceId: string) {
		edit((file) => void sc.removeScorePiece(file, part, pieceId));
		if (selected === pieceId) selected = null;
	}
	function reorder(from: number, to: number) {
		const next = sc.moveCollageRows(pieces, from, to);
		if (!next) return void (from !== to && toast(m['creator.collage_sides'](), { type: 'info' }));
		edit((file) => {
			const sheet = sc.sheetOf(file, part);
			if (sheet) sheet.pieces = sc.moveCollageRows(sheet.pieces, from, to) ?? sheet.pieces;
		});
	}
	function setPage(onPage: boolean) {
		const geo = pageOf(piece);
		const at = [...piecePath];
		edit((file) => {
			const p = pieceAt(file, at);
			if (p) sc.setPiecePage(p, onPage, geo);
		});
	}
	/** the file was replaced and the bars are checked: the collage now belongs to the part's file */
	function checked() {
		edit((file) => {
			const sheet = sc.sheetOf(file, part);
			const now = sc.partFile(file, part);
			if (sheet && now) sheet.file = now;
		});
	}

	// Delete / Backspace / Esc act on the live region only (DESIGN.md "Selection"): the stage, the focus on it or nowhere
	function onkeydown(e: KeyboardEvent) {
		if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey || isField(e.target) || picking) return;
		if (!liveRegion.is('score-stage') || !focusIn(e.target, stageEl)) return;
		if ((e.key === 'Delete' || e.key === 'Backspace') && piece) remove(piece.id);
		else if (e.key === 'Escape' && selected) selected = null;
		else return;
		e.preventDefault();
	}

	const validate = () => sc.stageProblems(item, part, bars, doc.entity, (bar) => m['creator.collage_missing_bar']({ bar }));
	const labels = scoreViewLabels();

	let stageEl = $state<HTMLElement>();
	const sideSize = panelWidth('sc-panel-width:score-stage', 22, () => stageEl?.getBoundingClientRect().right ?? window.innerWidth);
	sideSize.room = () => (stageEl?.clientWidth ?? window.innerWidth) / 2;
	usePageNav({
		fill: true,
		extra: () => [
			{ href: `/catalog/items?id=${encodeURIComponent(id)}`, label: name },
			{ href: `/catalog/items/${encodeURIComponent(id)}/collage`, label: m['creator.collage_stage']() }
		]
	});
</script>

<svelte:window {onkeydown} />

<MetaPage toon="" title={m['creator.collage_stage_title']({ name })} docs={[doc]} {validate} fill help="catalog">
	<div class="stage" bind:this={stageEl} style:--panel-width={sideSize.css} use:region={{ name: 'score-stage', initial: true }} data-testid="score-stage">
		<section class="main">
			<div class="bar">
				<Segmented
					label={m['creator.strip_preview_screen']()}
					square
					radio
					value={screen}
					onchange={pickScreen}
					options={[
						{ value: 'phone', icon: 'smartphone', title: m['creator.strip_preview_phone'](), testid: 'stage-phone' },
						{ value: 'desktop', icon: 'monitor', title: m['creator.strip_preview_desktop'](), testid: 'stage-desktop' }
					]}
					testid="stage-screen"
				/>
			</div>
			<div class="room">
				{#if item && !file}
					<p class="sc-empty" data-testid="stage-no-score">{m['creator.collage_stage_no_score']()}</p>
				{:else if unsigned}
					<p class="sc-empty" data-testid="stage-unsigned">{m['creator.collage_stage_unsigned']()}</p>
				{:else if src}
					<ScoreFrame {screen}>
						<ScoreView {src} title={name} {labels} {pieces} pieceSrc={asset} embedded bind:zoom bind:page {follow} {onlayout}>
							{#snippet decor(n, geometry)}
								{#if geometry}
									<ScoreOverlay {pieces} {geometry} {selected} onselect={(pieceId) => (selected = pieceId)} onmove={move} onscale={scale} ongeometry={(geo) => geometries.set(n, geo)} label={(p) => pieceLabel(p, registryOf(registry), moodsOf(moods))} scaleLabel={m['creator.collage_scale']()} />
								{/if}
							{/snippet}
						</ScoreView>
					</ScoreFrame>
				{/if}
			</div>
		</section>
		<Divider size={sideSize} edge="left" at="var(--panel-width, 22rem)" label={m['creator.resize_panel']()} controls="score-stage-side" class="side-resize" />
		<aside class="side" id="score-stage-side" aria-label={m['creator.inspector']()} data-testid="score-stage-side">
			{#if sc.fileChanged(item, part)}
				<p class="notice" role="status" data-testid="stage-file-changed">
					<span>{m['creator.collage_file_changed']()}</span>
					<button type="button" class="secondary" onclick={checked} data-testid="stage-file-checked">{m['creator.collage_checked']()}</button>
				</p>
			{/if}
			<CollageList {pieces} {selected} {missing} registry={registryOf(registry)} moods={moodsOf(moods)} onselect={(pieceId) => (selected = pieceId)} onmove={reorder} />
			<div class="adds">
				<button type="button" class="secondary" disabled={!src || !bars} onclick={() => (picking = true)} data-testid="stage-add-piece">{m['creator.add_slice_piece']()}</button>
			</div>
			{#if piece}
				<PieceInspector {doc} path={piecePath} host="score" registry={registryOf(registry)} moods={moodsOf(moods)} onremove={() => remove(piece.id)} onpage={setPage} />
			{/if}
		</aside>
	</div>
	<CollagePickerSheet bind:open={picking} registry={registryOf(registry)} moods={moodsOf(moods)} {toons} onpick={add} />
</MetaPage>

<style>
	/* the stage | the side region (its Divider drags it, agents/DOCKING.md) */
	.stage {
		position: relative;
		display: grid;
		grid-template-columns: minmax(0, 1fr) var(--panel-width, 22rem);
		grid-template-rows: minmax(0, 1fr);
		height: 100%;
		min-height: 0;
	}
	.main {
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
	}
	.bar {
		display: flex;
		justify-content: flex-end;
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.room {
		flex: 1;
		min-height: 0;
		padding: var(--space-1);
	}
	.side {
		min-width: 0;
		min-height: 0;
		overflow: auto;
		overscroll-behavior: contain;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-1);
		border-left: var(--border-width-regular) solid var(--ink);
	}
	.adds {
		display: flex;
		gap: var(--space-05);
	}
	.notice {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05);
		margin: 0;
		color: var(--state-warning);
	}
	/* narrower (a sidebar + 22rem + a phone screen need ≈ 1280px): the side region below the stage */
	@media (max-width: 1279px) {
		.stage {
			grid-template-columns: minmax(0, 1fr);
			grid-template-rows: minmax(0, 1fr) minmax(0, 45%);
		}
		.side {
			border-left: 0;
			border-top: var(--border-width-regular) solid var(--ink);
		}
		.stage :global(.side-resize) {
			display: none;
		}
	}
</style>
