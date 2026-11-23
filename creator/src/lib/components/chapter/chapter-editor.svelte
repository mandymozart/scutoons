<script lang="ts">
	import type { VariantMedia } from '@scutoons/media';
	import { onMount, tick, untrack } from 'svelte';
	import { Icon, confirmDialog } from '@scutoons/ui';
	import { liveRegion, region } from '@scutoons/ui';
	import { replaceState } from '$app/navigation';
	import { page } from '$app/state';
	import { usePageNav } from '$lib/nav.svelte.ts';
	import { usePageDoc, useTabs } from '$lib/shell/shell.svelte.ts';
	import { fileLabel } from '$lib/labels.ts';
	import { DocSession, useAutosave } from '$lib/editor/doc-session.svelte.ts';
	import { editorKeys, isField } from '$lib/editor/editor-keys.ts';
	import DocNotices from '$lib/components/page/doc-notices.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { toonDoc } from '$lib/editor/languages.ts';
	import { onLibrarySet, shareUses } from '$lib/editor/library-hook.svelte.ts';
	import { toonAssetRef } from '@scutoons/content';
	import { entities, getAt, deleteAt, scriptLabels, type Path, type ToonLanguages } from '@scutoons/content';
	import { castEntries, castIndex, chapterSceneCharacters, characterIndex, expressionIndex, sceneCastSuggestions, validateCast } from '@scutoons/content';
	import {
		addToSeasonCast,
		castDoc,
		castOf,
		castWithStar,
		loadToons,
		moodsDoc,
		moodsOf,
		registryDoc,
		registryOf,
		type ToonInfo
	} from '$lib/editor/characters.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import type { CharacterChoices } from '$lib/components/characters/character-ref-field.svelte';
	import { knownImages } from '$lib/editor/images.ts';
	import { seasonStatusOf, syncChapter } from '$lib/editor/visibility.ts';
	import { validateFile, validateNeutral, chapterReach, toonFacts, factNames, type ChapterReach, type FlowNode, type Problem, type ToonFacts } from '@scutoons/content';
	import { loadIncoming, loadOtherChapters, loadToonDocs } from '$lib/editor/toon-facts.ts';
	import { applyGroupRename, groupRenameProblem, planGroupRename } from '$lib/editor/group-rename.ts';
	import { JumpPlaces, setJumpPlaces } from '$lib/editor/jump-places.svelte.ts';
	import type { ReachMark } from '$lib/components/chapter/reach-chip.svelte';
	import { type Tool } from '$lib/components/slice/slice-canvas.svelte';
	import StripCanvas from '$lib/components/slice/strip-canvas.svelte';
	import { piecePlace } from '$lib/editor/canvas-pieces.ts';
	import type { PickedPiece } from '$lib/editor/collage-pick.ts';
	import { followPieces, piecesMoved } from '$lib/editor/collage-edit.ts';
	import { pieceLabel } from '$lib/editor/collage-words.ts';
	import { legacyTextCount, legacyPlan, applyLegacyPlan, type TextPreset } from '$lib/editor/text-presets.ts';
	import { itemCatalog } from '$lib/editor/catalog.svelte.ts';
	import { ChapterView, clampZoom, type InspectorTab, type Section } from '$lib/editor/chapter-state.svelte.ts';
	import * as edits from '$lib/editor/slice-edit.ts';
	import { clipboard, clipboardEvents, useClipHandler } from '$lib/editor/clipboard.svelte.ts';
	import { sliceClipHandler } from '$lib/editor/slice-clip.ts';
	import { setClipContext } from '$lib/editor/clip-context.ts';
	import { countPlacements } from '$lib/editor/clip.ts';
	import { itemName } from '$lib/editor/catalog.svelte.ts';
	import { api, saveBatch } from '$lib/api.ts';
	import type { ClipResult } from '$lib/editor/clipboard.svelte.ts';
	import ChapterTree from '$lib/components/chapter/chapter-tree.svelte';
	import SliceToolbar from '$lib/components/slice/slice-toolbar.svelte';
	import SliceInspector from '$lib/components/slice/slice-inspector.svelte';
	import ItemPickerSheet from '$lib/components/items/item-picker-sheet.svelte';
	import { pickPiece } from '$lib/editor/library-drop.ts';
	import { mapDoc as worldMapDoc } from '$lib/editor/map.ts';
	import { judgedPresets, ownPresets, presetsDoc } from '$lib/editor/presets-doc.ts';
	import MetaPanel from '$lib/components/chapter/meta-panel.svelte';
	import GroupPanel from '$lib/components/chapter/group-panel.svelte';
	import AudioPanel from '$lib/components/chapter/audio-panel.svelte';
	import VnEvents from '$lib/components/events/vn-events.svelte';
	import { isAccepted } from '$lib/editor/accepted.ts';
	import { useProblems } from '$lib/log/problems.svelte.ts';
	import { problemMessage, problemPlace } from '$lib/log/problem-words.ts';
	import { log } from '$lib/log/log.svelte.ts';
	import FlowTab from '$lib/components/flow/flow-tab.svelte';
	import PreviewTab from '$lib/components/reader/preview-tab.svelte';
	import ClipMenu from '$lib/components/page/clip-menu.svelte';
	import { toast } from '$lib/toasts.ts';
	import { shownCharacterName, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { exprContext, fieldKey } from '$lib/editor/expr-context.svelte.ts';
	import { chapterFlow } from '@scutoons/content';
	import { sporeScope } from '$lib/spores/scope.svelte.ts';
	import { storyWeave } from '$lib/spores/story.ts';

	/** `languages`: the toon's source + languages (ToonIndex, loaded by the page) – every file is edited in these */
	let { toon, chapterNumber, languages }: { toon: string; chapterNumber: number; languages: ToonLanguages } = $props();


	// ── Documents ───────────────────────────────────────────────────────────────────────────────────────
	// the page re-creates this component per chapter and toon languages ({#key}), so the initial props are the right ones
	// svelte-ignore state_referenced_locally
	const chapter = toonDoc(entities.chapter(toon, chapterNumber), languages);
	shareUses(() => `chapter:${toon}:${chapterNumber}`, () => (chapter.loading ? null : chapter.data[chapter.primary]), () => toon);
	onLibrarySet('thumbnail', () => `chapter:${toon}:${chapterNumber}`, (path) => chapter.edit((file) => void (file.thumbnail = toonAssetRef(toon, path))));
	// the season holds the chapter's status (visibility, per locale): edited in the meta panel, saved with the chapter
	// svelte-ignore state_referenced_locally
	const season = toonDoc(entities.season(toon, 1), languages);
	// the season's cast + the character registry: the VN character fields pick from every character (the season's cast
	// first); picking someone who isn't cast adds them to the season's cast (saved / published with the chapter)
	/** the chapter's season (the editor edits season 1) */
	const SEASON = 1;
	// svelte-ignore state_referenced_locally
	const cast = castDoc(toon);
	const registry = registryDoc();
	const moods = moodsDoc();
	// the custom collage presets (research/COLLAGE.md §12.4): the canvas draws, the fields edit, the checks judge the
	// pieces with its newest draft; saved / published with the chapter while edited here
	const colorations = presetsDoc();
	const collagePresets = $derived(ownPresets(colorations));
	// the world map: read for the expressions' map facts (places are unlocked by unlockLocation events – owner 2026-10-11)
	const worldMap = worldMapDoc();
	/** the world map's place ids (unlockLocation events point at them); undefined until it is loaded = no check */
	const worldMapPlaces = $derived.by(() => {
		const map = worldMap.loading ? null : (worldMap.data?.[worldMap.primary] as { locations?: { id: string; toons?: string[] }[] } | undefined);
		return map?.locations ? map.locations : undefined;
	});
	/** the world map's region ids (migrate events point at them); undefined until loaded = no check */
	const worldMapRegions = $derived.by(() => {
		const map = worldMap.loading ? null : (worldMap.data?.[worldMap.primary] as { regions?: { id: string }[] } | undefined);
		return map?.regions ? map.regions.map((r) => r.id) : undefined;
	});
	let toonInfos = $state<ToonInfo[]>([]);
	let castTouched = $state(false);
	const registryIndex = $derived(characterIndex(registryOf(registry)));
	const hasCast = $derived(!!castOf(cast) && !!registryOf(registry));
	const seasonCast = $derived(castEntries(castOf(cast), SEASON));
	const characters = $derived<{ name: string; label?: string }[]>(
		hasCast ? seasonCast.map((e) => ({ name: e.character, label: shownCharacterName(registryIndex.get(e.character)) || e.character })) : []
	);
	function ensureCast(character: string, persona: string) {
		const changed = addToSeasonCast(cast, SEASON, character, persona);
		if (!changed) return;
		castTouched = true;
		// a toast with Undo (E.1): the cast change is undone, the chapter edit stays
		toast(m['creator.characters_cast_added']({ name: shownCharacterName(registryIndex.get(character), persona) || character, season: m['creator.season']({ number: SEASON }) }), {
			action: { label: m['creator.undo'](), run: () => cast.undo() }
		});
	}
	const choices = $derived<CharacterChoices | undefined>(
		registry.loading || cast.loading
			? undefined
			: {
					registry: registryOf(registry),
					moods: moodsOf(moods),
					toons: toonInfos,
					cast: seasonCast,
					season: m['creator.season']({ number: SEASON }),
					// names, looks and moods in the header's text language (BACKLOG 10.51)
					locale: textLanguage.names,
					ensureCast
				}
	);
	let images = $state<string[]>([]);

	const chapterData = $derived(chapter.data[chapter.primary]);
	const groups = $derived<any[]>(chapterData?.groups ?? []);
	/**
	 * A slice sample picked as files (asset picker / upload; `src` = the 128k default, `variants` = its levels): its
	 * sample-library id. An entry with that file gets the levels; a file not in the chapter's library yet is added to it
	 * (id from the file name: s1c1-choir-loud.128.mp3 → choir_loud) – saved with the chapter.
	 */
	function librarySample(media: VariantMedia): string {
		const path = media.src ?? '';
		const library: any[] = chapterData?.audio?.sampleLibrary ?? [];
		const known = library.find((s) => s?.path === path || (media.variants && Object.values(s?.variants ?? {}).includes(path)));
		if (known?.id) {
			chapter.edit((file) => {
				const entry = (file.audio?.sampleLibrary ?? []).find((s: any) => s?.id === known.id);
				if (!entry) return;
				entry.path = path;
				if (media.variants) entry.variants = media.variants;
				else delete entry.variants;
			});
			return known.id;
		}
		const base = (path.split('/').pop() ?? path).replace(/(\.\d+)?\.[a-z0-9]+$/i, '').replace(/^s\d+c\d+-/i, '').replace(/[^a-z0-9]+/gi, '_').replace(/^_+|_+$/g, '').toLowerCase() || 'sample';
		let id = base;
		for (let n = 2; library.some((s) => s?.id === id); n++) id = `${base}_${n}`;
		chapter.edit((file) => {
			file.audio ??= {};
			file.audio.sampleLibrary ??= [];
			if (!file.audio.sampleLibrary.some((s: any) => s?.id === id)) file.audio.sampleLibrary.push(media.variants ? { id, path, variants: media.variants } : { id, path });
		});
		return id;
	}
	/**
	 * The music level each track holds when the active slice comes into view: the last value an earlier slice set (the
	 * reader keeps a track's level until a slice changes it; every track starts silent – AdaptiveMusicPlayer).
	 */
	const heldMusic = $derived.by(() => {
		const out: Record<string, { volume: number; label: string }> = {};
		if (view.section !== 'slice' || view.sliceIndex < 0) return out;
		const all = groups;
		for (let gi = 0; gi <= view.groupIndex && gi < all.length; gi++) {
			const slices: any[] = all[gi]?.slices ?? [];
			const until = gi === view.groupIndex ? view.sliceIndex : slices.length;
			for (let si = 0; si < until; si++) {
				for (const t of slices[si]?.audio?.music?.tracks ?? []) {
					if (t?.id && typeof t.volume === 'number') out[t.id] = { volume: t.volume, label: gi === view.groupIndex ? `slice ${si + 1}` : `${groupName(groups[gi], gi)} · ${si + 1}` };
				}
			}
		}
		return out;
	});
	const allDocs = $derived<ContentDoc[]>([
		chapter,
		season,
		...(castTouched || cast.dirty ? [cast] : []),
		...(worldMap.dirty ? [worldMap] : []),
		...(colorations.dirty ? [colorations] : [])
	]);
	/**
	 * the Recover bars' documents: the page's, and the presets while clean – autosaved only while dirty, so a reload or a
	 * crash leaves a copy that the load offers next to the chapter's, whose pieces may name its ids (research/COLLAGE.md
	 * §12.4); not in allDocs: a never-stored presets document is not "missing"
	 */
	const noticeDocs = $derived(colorations.dirty ? allDocs : [...allDocs, colorations]);
	/** the chapter's documents with slices (one document per chapter since 2026-10-19) */
	const sliceDocs = $derived<ContentDoc[]>([chapter]);
	/** texts that still have the old look (a mask over baked text, chapter 1) */
	const legacyTexts = $derived(sliceDocs.reduce((n, d) => n + (d.loading ? 0 : legacyTextCount(d.data[d.primary])), 0));
	/** one step (owner 2026-10-03): every masked text → its preset; save / publish as usual */
	function convertLegacyTexts() {
		for (const d of sliceDocs) {
			const plan = legacyPlan($state.snapshot(d.data[d.primary]));
			if (plan.length) d.edit((file) => applyLegacyPlan(file, plan), 'convert-presets');
		}
	}
	const anySaving = $derived(allDocs.some((d) => d.saving));

	onMount(async () => {
		await Promise.all([chapter.load(), season.load(), cast.load(), registry.load(), moods.load(), worldMap.load(), colorations.load()]);
		loadToons(getLocale()).then((list) => (toonInfos = list));
		refreshImages();
		// the rest of the toon (reachability: labels something jumps to, variables something sets) – without it no
		// condition is judged and every labelled group counts as reachable
		// + the labels other toons jump to (the server's flow index)
		Promise.all([loadToonDocs(toon, new Set(sliceDocs.map((d) => d.entity))), loadIncoming(toon)])
			.then(([docs, incoming]) => (toonDocsFacts = toonFacts(docs, { toon, incoming: incoming.labels, elsewhere: incoming.elsewhere })))
			.catch(() => (toonDocsFacts = null));
	});

	// ── Reachability (owner 2026-10-06, @scutoons/content reach.ts) ─────────────────────────────────────────────────
	let toonDocsFacts = $state<ToonFacts | null>(null);
	/** the toon's facts: the other documents (loaded once) + this chapter's own, live */
	const facts = $derived.by<ToonFacts | undefined>(() => {
		const rest = toonDocsFacts;
		if (!rest) return undefined;
		const live = toonFacts([...sliceDocs.map((d) => $state.snapshot(d.data[d.primary])), $state.snapshot(worldMap.data?.[worldMap.primary])], { toon });
		return { toon, variables: new Set([...rest.variables, ...live.variables]), labels: new Set([...rest.labels, ...live.labels]), ...(rest.reads ? { reads: rest.reads } : {}) };
	});
	// the expression and variable fields: the story variables of every toon as written here (`x`, `global.x`,
	// `toptoy.x` – one save game; the builder and the variable typeahead offer them, unknown names warn) and the toons
	$effect(() => exprContext.setVariables(facts ? factNames(facts) : null));
	$effect(() => {
		exprContext.toon = toon;
		exprContext.toons = toonInfos.map((t) => ({ id: t.id, name: t.name }));
	});
	$effect(() => () => exprContext.setVariables(null));
	/** the chapter's reachability for the labels in the tree / slice list / event lists – debounced while editing */
	let reach = $state<ChapterReach | null>(null);
	/** the groups `reach` was computed from (the Flow panel draws exactly what the chips say) */
	let reachGroups = $state.raw<any[] | null>(null);
	$effect(() => {
		const snapshot = $state.snapshot(groups);
		const f = facts;
		const timer = setTimeout(() => {
			reach = chapterReach(snapshot, f);
			reachGroups = snapshot;
		}, 250);
		return () => clearTimeout(timer);
	});
	/** chapter path (groups.g.…) → its mark */
	const marks = $derived.by(() => {
		const out = new Map<string, ReachMark>();
		const r = reach;
		if (!r) return out;
		r.groups.forEach((x, gi) => x === 'unreachable' && out.set(`groups.${gi}`, 'unreachable'));
		r.slices.forEach((list, gi) => list.forEach((x, si) => x === 'unreachable' && r.groups[gi] !== 'unreachable' && out.set(`groups.${gi}.slices.${si}`, 'unreachable')));
		r.events.forEach((list, gi) => list.forEach((x, i) => x === 'unreachable' && r.groups[gi] !== 'unreachable' && out.set(`groups.${gi}.events.${i}`, 'unreachable')));
		for (const p of r.loops) out.set(p.join('.'), 'loop');
		for (const p of r.neverTrue) out.set(p.slice(0, -1).join('.'), 'never');
		for (const p of r.stuck ?? []) out.set(p.join('.'), 'stuck');
		return out;
	});
	const markAt = (...path: (string | number)[]): ReachMark => marks.get(path.join('.')) ?? null;

	function refreshImages() {
		knownImages(toon, chapter.textLanguages, allDocs.map((d) => $state.snapshot(d.data))).then((list) => (images = list));
	}

	// ── Selection / view state: one object (lib/editor/chapter-state.svelte.ts) – the tree, the canvas, the inspector
	// and the keyboard write it
	const view = new ChapterView();
	// the canvas and the inspector show the top bar's TEXT language (default the source; owner
	// 2026-10-06: one language everywhere, the header's switch the only one)
	const displayLocale = $derived(textLanguage.current(languages.languages, languages.source));
	// snap, grid size, zoom and the seams toggle are remembered on this device
	$effect(() => {
		void view.snap;
		void view.gridSize;
		void view.zoom;
		void view.seams;
		view.savePrefs();
	});
	// the Flow, the Preview (the reader) and Problems are panel tabs (lib/shell/panels.ts)
	const flowProps = () => ({
		input: {
			level: 'chapter' as const,
			groups: reachGroups,
			reach,
			chapter: chapterNumber,
			current: view.section === 'group' || view.section === 'slice' ? view.groupIndex : -1,
			slice: view.section === 'slice' ? view.sliceIndex : -1,
			who: flowWho,
			toon,
			registry: registryOf(registry),
			facts,
			nextChapter: flowNextChapter,
			name: flowName,
			text: flowText,
			onopen: openFlowNode
		}
	});
	// the Spores tab (#763): this chapter's story as a weave, offered while the chapter is open
	const sporesAddress = $derived(`chapter:${toon}/${chapterNumber}`);
	$effect(() => {
		const address = sporesAddress;
		sporeScope.set({
			address,
			weave: () => {
				const p = flowProps().input;
				if (!p.groups || !p.reach) return null;
				const flow = chapterFlow(p.groups, { chapter: p.chapter, toon: p.toon, reach: p.reach, name: p.name, text: p.text, facts: p.facts, nextChapter: p.nextChapter });
				const levels = [
					{ address: 'network', label: 'SCUTOONS' },
					{ address: `toon:${p.toon}`, label: p.toon },
					{ address, label: `${p.chapter}` }
				];
				return storyWeave(flow, { context: address, levels });
			}
		});
		return () => sporeScope.drop(address);
	});
	// the Reader (UI-CONCEPTS E.6): the viewer's draft preview at the open group / slice (the chapter's start on its meta /
	// audio), in the text language when the reader has it (de / en), else the first it has
	const readerLocale = $derived(displayLocale === 'de' || displayLocale === 'en' ? displayLocale : (chapter.textLanguages.find((l) => l === 'de' || l === 'en') ?? 'de'));
	const readerProps = () => ({
		place: {
			toon,
			chapter: chapterNumber,
			group: view.section === 'group' || view.section === 'slice' ? Math.max(0, view.groupIndex) : 0,
			slice: view.section === 'slice' ? view.sliceIndex : -1
		},
		locale: readerLocale
	});
	const groupLabel = (i: number) => groupName(groups[i], i);
	useProblems(() => ({ problems, docs: allDocs, home: chapter.entity, place: (p) => problemPlace(p, chapter.entity, groupLabel), message: problemMessage, ongoto: gotoProblem }));
	// the Log's Selection scope: the picked group or slice, as a problem's path inside the chapter
	$effect(() => {
		const item = log.item;
		const group = view.section === 'group' || view.section === 'slice' ? view.groupIndex : -1;
		log.selection = item && group >= 0 ? `${item}/groups/${group}${view.section === 'slice' && view.sliceIndex >= 0 ? `/slices/${view.sliceIndex}` : ''}` : null;
		return () => (log.selection = null);
	});
	useTabs(() => ({ tree, flow: flowTab, preview: previewTab, inspector: view.section === 'slice' && activeDoc && activeSlice ? inspectorTab : null }));
	let canvas = $state<StripCanvas | null>(null);
	/**
	 * Ctrl / Cmd + wheel over the slice area zooms the canvas (also with the zoom flyout closed; a trackpad's pinch is the
	 * same event), 50–300 %, the point under the pointer stays put (UI-CONCEPTS E.4). Non-passive: it replaces the
	 * browser's page zoom there.
	 */
	function wheelZoom(area: HTMLElement) {
		const onWheel = async (event: WheelEvent) => {
			if (!event.ctrlKey && !event.metaKey) return;
			event.preventDefault();
			const el = area.querySelector<HTMLElement>('[data-testid="canvas-wrap"]');
			if (!el) return;
			const before = el.getBoundingClientRect();
			const fx = (event.clientX - before.left) / (before.width || 1);
			const fy = (event.clientY - before.top) / (before.height || 1);
			const step = event.deltaMode === 1 ? event.deltaY * 16 : event.deltaY;
			const next = clampZoom(view.shownZoom * Math.exp(-step * 0.002));
			if (next === view.shownZoom && view.zoom !== null) return;
			view.zoom = next;
			view.shownZoom = next;
			await tick();
			const after = el.getBoundingClientRect();
			area.scrollLeft += after.left + fx * after.width - event.clientX;
			area.scrollTop += after.top + fy * after.height - event.clientY;
		};
		area.addEventListener('wheel', onWheel, { passive: false });
		const offPinch = pinchZoom(area);
		return () => {
			area.removeEventListener('wheel', onWheel);
			offPinch();
		};
	}
	/**
	 * Two fingers on the slice area zoom it (owner 2026-10-21: "zoom inside the slices stage to do things carefully in
	 * detail. Finger gestures should apply"): the same 50–300 % as Ctrl + wheel, the point between the fingers stays put;
	 * the second finger drops the drag / draw the first one started. Non-passive: it replaces the browser's page zoom.
	 */
	function pinchZoom(area: HTMLElement) {
		let pinch: { d0: number; z0: number } | null = null;
		const dist = (t: TouchList) => Math.hypot(t[0].clientX - t[1].clientX, t[0].clientY - t[1].clientY);
		const centre = (t: TouchList) => ({ x: (t[0].clientX + t[1].clientX) / 2, y: (t[0].clientY + t[1].clientY) / 2 });
		/** the element that scrolls vertically */
		const vScroller = () => {
			for (let el: HTMLElement | null = area; el; el = el.parentElement) {
				const o = getComputedStyle(el).overflowY;
				if ((o === 'auto' || o === 'scroll') && el.scrollHeight > el.clientHeight) return el;
			}
			return (document.scrollingElement as HTMLElement | null) ?? document.documentElement;
		};
		const onStart = (event: TouchEvent) => {
			if (event.touches.length !== 2) return;
			canvas?.cancelGesture();
			pinch = { d0: dist(event.touches) || 1, z0: view.shownZoom };
		};
		const onMove = async (event: TouchEvent) => {
			if (!pinch || event.touches.length !== 2) return;
			event.preventDefault();
			const el = area.querySelector<HTMLElement>('[data-testid="canvas-wrap"]');
			if (!el) return;
			const c = centre(event.touches);
			const before = el.getBoundingClientRect();
			const fx = (c.x - before.left) / (before.width || 1);
			const fy = (c.y - before.top) / (before.height || 1);
			const next = clampZoom(pinch.z0 * (dist(event.touches) / pinch.d0));
			if (next === view.shownZoom && view.zoom !== null) return;
			view.zoom = next;
			view.shownZoom = next;
			await tick();
			const after = el.getBoundingClientRect();
			area.scrollLeft += after.left + fx * after.width - c.x;
			vScroller().scrollTop += after.top + fy * after.height - c.y;
		};
		const onEnd = (event: TouchEvent) => {
			if (event.touches.length < 2) pinch = null;
		};
		area.addEventListener('touchstart', onStart, { passive: true });
		area.addEventListener('touchmove', onMove, { passive: false });
		area.addEventListener('touchend', onEnd);
		area.addEventListener('touchcancel', onEnd);
		return () => {
			area.removeEventListener('touchstart', onStart);
			area.removeEventListener('touchmove', onMove);
			area.removeEventListener('touchend', onEnd);
			area.removeEventListener('touchcancel', onEnd);
		};
	}
	/** autosave / publish / conflict / the notice (lib/editor/doc-session.svelte.ts – the same as the meta pages') */
	const session = new DocSession({
		docs: () => allDocs,
		beforeSave: () => {
			// the chapter file's (deprecated) status mirrors the season, its old `published` flag goes
			syncChapter(chapter);
			// a cast saved from here keeps the toon's star in every season
			if (castTouched || cast.dirty) castWithStar(cast, registry, toon, [SEASON]);
		},
		afterSave: () => refreshImages(),
		errorCount: () => errorCount
	});
	useAutosave(session);

	/** the edited document: the chapter (its groups inline – the scene documents went 2026-10-19) */
	const activeDoc = $derived<ContentDoc | null>(chapter);
	/** undo / redo act on the chapter */
	const undoDoc = $derived<ContentDoc | null>(chapter.canUndo ? chapter : null);
	const redoDoc = $derived<ContentDoc | null>(chapter.canRedo ? chapter : null);
	/**
	 * undo / redo may put the pieces back in another order (a Layers reorder) or take one away: the selected piece follows
	 * its id (gone: let go) – the text boxes / items keep their indices (their order is `layer`)
	 */
	function keepPiece(step: () => void) {
		// the list as it was: undo / redo replace the document, they never change it in place
		const before = view.selection?.kind === 'piece' ? activeSlice?.pieces : undefined;
		step();
		if (before) view.selection = followPieces(view.selection, piecesMoved(before, activeSlice?.pieces));
	}
	function undoLast() {
		const d = undoDoc;
		if (!d) return;
		keepPiece(() => d.undo());
	}
	const redoLast = () => keepPiece(() => redoDoc?.redo());
	const groupPath = $derived<Path>(['groups', view.groupIndex]);
	const slicePath = $derived<Path>([...groupPath, 'slices', view.sliceIndex]);
	const activeGroup = $derived(activeDoc?.get(activeDoc.primary, groupPath));
	const activeSlice = $derived(view.sliceIndex >= 0 ? activeDoc?.get(activeDoc.primary, slicePath) : null);
	const sliceDialogs = $derived<any[]>(activeSlice?.dialog ?? []);
	const sliceItems = $derived<any[]>(activeSlice?.items ?? []);

	// ── Navigation: the tree is the sidebar, group + slice are the deepest crumbs, the URL mirrors the selection ──
	// (`?group=<id>&slice=<n>`, `?section=audio`; deep links and Alt+ArrowUp / crumbs work through the URL)
	// the page re-creates this component per chapter ({#key})
	// svelte-ignore state_referenced_locally
	const chapterHref = `/${toon}/chapter/${chapterNumber}`;
	const groupKey = (g: any, i: number) => g?.id || String(i + 1);
	const groupName = (g: any, i: number) => g?.id || `${m['creator.groups']()} ${i + 1}`;
	/** what is not live yet: the chapter, the season while it has something unpublished */
	const liveFiles = $derived([
		{ doc: chapter, label: fileLabel(chapter.entity) },
		...(!season.loading && (season.dirty || season.unpublished.length) ? [{ doc: season, label: fileLabel(season.entity) }] : [])
	]);
	function hrefFor(next: { section: Section; group?: number; slice?: number }) {
		const params = new URLSearchParams();
		if (next.section === 'audio') params.set('section', 'audio');
		if ((next.section === 'group' || next.section === 'slice') && next.group !== undefined) params.set('group', groupKey(groups[next.group], next.group));
		if (next.section === 'slice' && next.slice !== undefined && next.slice >= 0) params.set('slice', String(next.slice + 1));
		const query = params.toString();
		return query ? `${chapterHref}?${query}` : chapterHref;
	}
	usePageNav({
		fill: true,
		extra: () => {
			if (view.section !== 'group' && view.section !== 'slice') return [];
			const out = [{ href: hrefFor({ section: 'group', group: view.groupIndex }), label: groupName(groups[view.groupIndex], view.groupIndex) }];
			if (view.section === 'slice' && view.sliceIndex >= 0) out.push({ href: hrefFor({ section: 'slice', group: view.groupIndex, slice: view.sliceIndex }), label: m['creator.slice_n']({ number: view.sliceIndex + 1 }) });
			return out;
		}
	});
	/** the URL has been applied once (after loading): only then does the selection write the URL */
	let urlApplied = $state(false);
	$effect(() => {
		// URL → selection (deep links, crumbs, back / forward)
		const params = page.url.searchParams;
		if (chapter.loading) return;
		const wantSection = params.get('section');
		const wantGroup = params.get('group');
		const wantSlice = Number(params.get('slice'));
		untrack(() => {
			if (wantGroup !== null) {
				let gi = groups.findIndex((g, i) => groupKey(g, i) === wantGroup);
				if (gi < 0 && /^\d+$/.test(wantGroup)) gi = Number(wantGroup) - 1;
				if (gi >= 0 && gi < groups.length) {
					if (gi !== view.groupIndex || (view.section !== 'group' && view.section !== 'slice')) view.selectGroup(gi);
					if (wantSlice > 0) {
						if (view.section !== 'slice' || view.sliceIndex !== wantSlice - 1) view.selectSlice(wantSlice - 1);
					} else if (view.section === 'slice') {
						view.selectGroup(gi);
					}
				}
			} else if (wantSection === 'audio') {
				view.section = 'audio';
			} else if (view.section !== 'meta') {
				view.section = 'meta';
			}
			urlApplied = true;
		});
	});
	$effect(() => {
		// selection → URL (shareable; a crumb goes back to the same place)
		const href = hrefFor({ section: view.section, group: view.groupIndex, slice: view.sliceIndex });
		if (!urlApplied) return;
		untrack(() => {
			// the address bar, not page.url: replaceState (shallow routing) leaves page.url at the last navigation, so
			// Audio → Meta compared with the stale URL and never cleared ?section=audio
			if (href !== location.pathname + location.search) {
				try {
					replaceState(href, {});
				} catch {
					/* router not ready */
				}
			}
		});
	});

	// ── Problems ────────────────────────────────────────────────────────────────────────────────────────
	const problems = $derived.by<Problem[]>(() => {
		const out: Problem[] = [];
		const names = characters.map((c) => c.name);
		const expressions = registryOf(registry) ? expressionIndex(registryOf(registry)) : undefined;
		const moodIds = moodsOf(moods).moods.map((x) => x.id);
		const catalogItems = itemCatalog.catalog ? itemCatalog.catalog.items.map((i) => i.id) : undefined;
		const itemRespawn = itemCatalog.catalog ? Object.fromEntries(itemCatalog.catalog.items.map((i) => [i.id, i.respawn ?? null])) : undefined;
		const presets = judgedPresets(colorations);
		for (const doc of allDocs) {
			if (doc.loading) continue;
			const data = doc.data[doc.primary];
			const ctx = {
				file: doc.entity,
				audio: chapterData?.audio,
				characters: !hasCast && names.length ? names : undefined,
				cast: hasCast ? castIndex(castOf(cast), SEASON) : undefined,
				expressions,
				moods: moodIds,
				// collage pieces: a custom preset is judged once the presets are loaded
				presets,
				// the chapter's visibility: its entry in the season (null = none)
				seasonStatus: doc === chapter ? seasonStatusOf(season, languages.source, chapterNumber) : undefined,
				// reachability: what the rest of the toon jumps to / sets
				facts,
				// jump targets: the toon / chapter / label they name exist (as far as loaded)
				targets: jumpPlaces.targets(),
				// placed items (and related items) are checked against the catalog once it is loaded
				items: catalogItems,
				itemRespawn,
				// unlockLocation: the world map's places (once loaded)
				places: worldMapPlaces?.map((l) => l.id),
				toonPlaces: worldMapPlaces?.filter((l) => l.toons?.includes(toon)).map((l) => l.id),
				regions: worldMapRegions
			};
			if (data == null) {
				if (doc.source[doc.primary] === 'missing') out.push({ level: 'error', file: doc.entity, locale: null, path: [], message: m['creator.missing']() });
				continue;
			}
			// chapter, season: one language-neutral document, checked once + its texts per language
			out.push(...(doc.texts ? validateNeutral(doc.kind, data, doc.texts, ctx) : validateFile(doc.kind, data, doc.primary, ctx)));
		}
		// item limits (BACKLOG 10.39): an item placed more often than the catalog allows, every toon together
		for (const [id, c] of placementCounts) {
			const total = c.here + c.others.reduce((sum, p) => sum + p.count, 0);
			if (total <= c.limit || !c.here) continue;
			const at = placementPath(id);
			if (!at) continue;
			const item = itemCatalog.catalog?.items.find((i) => i.id === id);
			out.push({ level: 'error', file: at.file, locale: null, path: at.path, message: m['creator.clip_limit_problem']({ item: itemName(item, id), count: total, limit: c.limit, where: placesText(c.others) }) });
		}
		if (castTouched && castOf(cast)) out.push(...validateCast(castOf(cast), { registry: registryOf(registry), languages: languages.languages, file: entities.cast(toon) }));
		// suggestions from the VN usage (info): who the scenes use but the season cast lacks
		if (!chapter.loading && hasCast) out.push(...sceneCastSuggestions(chapterSceneCharacters(groups), castIndex(castOf(cast), SEASON), chapter.entity));
		const order = { error: 0, warning: 1, info: 2 };
		return out.sort((a, b) => order[a.level] - order[b.level]);
	});
	const errorCount = $derived(problems.filter((p) => p.level === 'error').length);
	/** the first placement of an item in this chapter's documents (a problem's place) */
	function placementPath(id: string): { file: string; path: Path } | null {
		for (const d of sliceDocs) {
			const data = d.data[d.primary];
			if (!data) continue;
			const groupsOf: [Path, any][] = Array.isArray(data.groups) ? data.groups.map((g: any, gi: number) => [['groups', gi], g]) : [[[], data]];
			for (const [base, g] of groupsOf)
				for (const [si, s] of (g?.slices ?? []).entries())
					for (const [ii, it] of (s?.items ?? []).entries()) if (it?.item === id) return { file: d.entity, path: [...base, 'slices', si, 'items', ii] };
		}
		return null;
	}
	// accepted warnings ("Looks fine") don't count
	const warningCount = $derived(problems.filter((p) => p.level === 'warning' && !isAccepted(allDocs, p)).length);

	/** a slice whose text boxes hold `{…}`: the canvas toolbar offers the sample readers */
	const sliceHasExpressions = (slice: any) =>
		(slice?.dialog ?? []).some((d: any) => {
			const t = d?.text;
			const all = t && typeof t === 'object' && !Array.isArray(t) ? Object.values(t) : [t];
			return all.flat().some((x) => typeof x === 'string' && x.includes('{'));
		});

	function gotoProblem(p: Problem) {
		// an expression's problem: its field (once shown) focuses and selects the range (expr-text.svelte)
		if (p.expr) exprContext.focus = { key: fieldKey(p.file, p.path, p.locale), line: p.expr.line, start: p.expr.start, end: p.expr.end };
		let path = p.path;
		if (p.file === season.entity) {
			view.section = 'meta';
			return;
		}
		if (p.file !== chapter.entity) return;
		if (path[0] === 'groups' && typeof path[1] === 'number') {
			view.selectGroup(path[1]);
			path = path.slice(2);
		} else {
			view.section = path[0] === 'audio' ? 'audio' : 'meta';
			return;
		}
		// a problem in another language switches the header's text language to it
		if (p.locale && p.locale !== displayLocale && (languages.languages as string[]).includes(p.locale)) textLanguage.set(p.locale);
		if (path[0] === 'slices' && typeof path[1] === 'number') {
			view.selectSlice(path[1]);
			const rest = path.slice(2);
			if (rest[0] === 'dialog' && typeof rest[1] === 'number') view.select({ kind: 'dialog', index: rest[1] });
			else if (rest[0] === 'items' && typeof rest[1] === 'number') view.select({ kind: 'item', index: rest[1] });
			else if (rest[0] === 'pieces' && typeof rest[1] === 'number') view.select({ kind: 'piece', index: rest[1] });
			else if (rest[0] === 'audio') view.inspectorTab = 'audio';
			else if (rest[0] === 'events') view.inspectorTab = 'events';
			else view.inspectorTab = 'slice';
		}
	}

	// ── Flow panel (owner 2026-10-06, read-only): a box opens its place in the editor – navigation only ──────────
	/** a group's name in the graph (the tree's) and a text in the header's language (a choice's label) */
	const flowName = (g: any, i: number) => groupName(groups[i] ?? g, i);
	const flowText = (v: unknown): string => {
		if (typeof v === 'string') return v;
		if (!v || typeof v !== 'object') return '';
		const map = v as Record<string, unknown>;
		const pick = [displayLocale, languages.source, ...Object.keys(map)].map((l) => map[l]).find((t) => typeof t === 'string' && t.trim());
		return typeof pick === 'string' ? pick : '';
	};
	/** a character key of the graph (`boy`, `catpope·cat`) as its name in the header's language */
	const flowWho = (key: string): string => {
		const [id, persona] = key.split('·');
		return shownCharacterName(registryIndex.get(id), persona) || key;
	};
	/** the chapter the follow-up bar leads to after the last group: the season's next one (the Flow's next-chapter picture) */
	const flowNextChapter = $derived.by(() => {
		const items: any[] = (season.get(season.primary, ['sequence']) as any[] | undefined) ?? [];
		const numbers = items.filter((i) => (i?.type ?? 'chapter') === 'chapter' && Number.isFinite(Number(i?.number))).map((i) => Number(i.number));
		const at = numbers.indexOf(chapterNumber);
		return at >= 0 ? numbers[at + 1] : undefined;
	});
	async function openFlowNode(node: FlowNode) {
		if (!node.path) return;
		gotoProblem({ level: 'info', file: chapter.entity, locale: null, path: node.path, message: '' });
		// a script's event: scroll its row into view (the group's VN list)
		const [, , list, index] = node.path;
		if (list === 'events' && typeof index === 'number') {
			await tick();
			document.querySelector(`[data-testid="vn-events"] [data-testid="event-${index}"]`)?.scrollIntoView({ block: 'center' });
		}
	}

	// ── Editing actions (lib/editor/slice-edit.ts) ────────────────────────────────────────────────────────────
	/** the chapter's group ids: a jump's group and the random list (CUTOVER 10.27) */
	const groupIds = () => groups.map((g: any) => g?.id).filter((id: unknown): id is string => typeof id === 'string' && !!id);
	const groupLabels = $derived(groupIds());
	// the places jumps lead to (the target editor's toon → chapter → label; validation checks targets against them)
	// svelte-ignore state_referenced_locally
	const jumpPlaces = setJumpPlaces(new JumpPlaces(toon, chapterNumber, groupIds));
	$effect(() => {
		jumpPlaces.toons = toonInfos.length ? toonInfos.map((t) => ({ id: t.id, name: t.name })) : null;
	});
	// load what the chapter's targets name (other chapters' / toons' labels) – once per target, debounced while editing
	$effect(() => {
		const docs = sliceDocs.map((d) => $state.snapshot(d.data[d.primary]));
		const timer = setTimeout(() => jumpPlaces.prefetch(docs), 500);
		return () => clearTimeout(timer);
	});
	/** every text box id of the chapter: a new box gets a fresh one (owner 2026-10-06: unique in the chapter) */
	const chapterDialogIds = () =>
		new Set<string>(groups.flatMap((g: any) => (g?.slices ?? []).flatMap((s: any) => (s?.dialog ?? []).map((d: any) => d?.id).filter((id: unknown) => typeof id === 'string'))));
	const sliceTarget = (): edits.SliceTarget | null =>
		activeDoc && view.sliceIndex >= 0 ? { doc: activeDoc, slicePath, chapterNumber, sliceIndex: view.sliceIndex, slice: activeSlice, taken: chapterDialogIds } : null;

	/**
	 * Renames a group's id along (group-rename.ts): the toon's other chapters that refer to it are listed with the open
	 * chapter's own references in an in-page confirmation, then saved in one batch; the open chapter changes in one edit step.
	 */
	async function renameGroup(from: string, to: string) {
		const problem = groupRenameProblem(groups, from, to);
		if (problem) return void toast(problem, { type: 'error' });
		const input = { toon, chapter: chapterNumber, from, to };
		try {
			const others = await loadOtherChapters(toon, chapter.entity);
			const plan = planGroupRename({ ...input, current: $state.snapshot(chapter.data[chapter.primary]) as any, others });
			if (plan.lines.length) {
				const list = plan.lines.map((l) => `· ${l}`).join('\n');
				const ok = await confirmDialog({ title: m['creator.group_rename_title']({ from, to }), message: m['creator.group_rename_message']({ list }), confirmLabel: m['creator.group_rename_confirm']() });
				if (!ok) return;
			}
			chapter.edit((file) => applyGroupRename(file, input));
			// the server checks a save's jumps against the newest versions: the group's own chapter goes first
			if (plan.saves.length) {
				await chapter.save(`group "${from}" → "${to}"`);
				await saveBatch(plan.saves, `group "${from}" → "${to}"`);
			}
			toast(m['creator.group_renamed']({ to, count: String(plan.lines.length) }));
		} catch (e) {
			toast(m['creator.group_rename_failed']({ error: e instanceof Error ? e.message : String(e) }), { type: 'error', timeout: 0 });
		}
	}
	/** a new text with a preset's look (default speech; a drawn one keeps the drawn rect); the cursor goes to its text */
	function addDialog(rect?: { x: number; y: number; width: number; height?: number }, preset?: TextPreset) {
		view.pickingPreset = false;
		const t = sliceTarget();
		if (!t) return;
		view.select(edits.addDialog(t, rect, preset));
		requestAnimationFrame(() => document.querySelector<HTMLTextAreaElement>('[data-testid="dialog-inspector"] textarea')?.focus());
	}
	function createBoxForEvent(index: number, eventsPath?: Path) {
		const t = sliceTarget();
		if (!t) return;
		// the new box is selected (the text box tab opens) with its text ready to type
		view.select(edits.createBoxForEvent(t, index, eventsPath));
		requestAnimationFrame(() => document.querySelector<HTMLTextAreaElement>('[data-testid="dialog-inspector"] textarea')?.focus());
	}
	function addSliceItem(itemId: string) {
		const spot = view.itemSpot;
		view.itemSpot = null;
		const t = sliceTarget();
		if (!t || !itemId) return;
		// the item's placement limit (BACKLOG 10.39): refused with a toast
		const refused = placementLimit(new Map([[itemId, 1]]));
		if (refused) return void toast(refused.message, { type: 'error', timeout: 0 });
		view.select(edits.addSliceItem(t, itemId, spot));
	}
	/** a picked collage piece at the piece tool's spot (research/COLLAGE.md §3: a click = centred there, a drag = its rect) */
	// the piece tool / "+ piece" asks the Library for a picture
	$effect(() => {
		if (!view.pickingPiece) return;
		view.pickingPiece = false;
		untrack(() => pickPiece(m['creator.collage_picker_title'](), choices?.registry?.characters ?? [], addPiece));
	});
	function addPiece(picked: PickedPiece) {
		const place = view.pieceSpot ?? piecePlace(null);
		view.pieceSpot = null;
		const t = sliceTarget();
		if (t) view.select(edits.addPiece(t, picked, place));
	}
	function addSliceEvent(rect: { x: number; y: number; width: number; height: number }) {
		const t = sliceTarget();
		if (t) view.select(edits.addSliceEvent(t, rect));
	}
	function addSampleArea(rect: { x: number; y: number; width: number; height: number }) {
		const t = sliceTarget();
		if (!t) return;
		const next = edits.addSampleArea(t, rect, chapterData?.audio?.sampleLibrary ?? []);
		if (next) view.select(next);
		else session.notify('The chapter has no sample library yet (Audio in the tree).');
	}
	/** what a tool makes of a drawn rect on the canvas (the item / piece / event tools: a click too, width 0) */
	const CREATE: Record<Tool, (rect: { x: number; y: number; width: number; height: number }) => void> = {
		text: (rect) => addDialog(rect),
		item: (rect) => view.pickItem({ x: rect.x, y: rect.y }),
		piece: (rect) => view.pickPiece(rect),
		sample: (rect) => addSampleArea(rect),
		event: (rect) => addSliceEvent(rect)
	};
	function removeSelected() {
		const t = sliceTarget();
		if (!t || !view.selection) return;
		edits.removeSelected(t, view.selection);
		view.selection = null;
	}

	/** copy / cut / paste on the slice (the live region), as the clipboard menu */
	async function clip(action: 'copy' | 'cut' | 'paste' | 'duplicate') {
		liveRegion.set(SLICE_REGION);
		if (action === 'paste') return void (await clipboard.pasteFromMenu(false));
		await clipboard.copyFromMenu(action === 'cut');
		if (action === 'duplicate') await clipboard.pasteFromMenu(false);
	}
	// the bar: ↶ ↷, Publish (its dot) and History; the status bar: "saving…" and the counts
	usePageDoc(() => ({
		saving: anySaving || session.saving,
		errors: errorCount,
		warnings: warningCount,
		unpublished: liveFiles.some((f) => f.doc.unpublished.length > 0),
		...session.publishHooks,
		docs: activeDoc && activeDoc !== chapter ? [activeDoc, chapter] : [chapter],
		undo: undoLast,
		redo: redoLast
	}));

	// ── The local copy, keyboard ────────────────────────────────────────────────────────────────────────
	$effect(() => {
		const docs = allDocs;
		for (const d of docs) void JSON.stringify(d.data);
		const timer = setTimeout(() => docs.forEach((d) => d.autosave()), 1000);
		return () => clearTimeout(timer);
	});

	/** the selection region of the slice canvas + its inspector (Layers, audio samples, slice events: one selection) */
	const SLICE_REGION = 'slice';

	// copy / cut / paste (BACKLOG 10.39, lib/editor/clipboard.svelte.ts): the DOM events act on the live region's handler;
	// the canvas region copies its selection and pastes into the open slice
	$effect(() => clipboardEvents());
	// ── item limits (BACKLOG 10.39 step 7, ITEMS.md "Item limits"): a catalog item's `placements` count over every toon –
	// this chapter's documents live, every other chapter from the API (GET /api/catalog/items/<id>/placements,
	// loaded once per limited item); over the limit = a Problems error, a paste that would go over it is refused
	let placedElsewhere = $state<Record<string, { key: string; count: number }[]>>({});
	$effect(() => {
		const items = itemCatalog.catalog?.items ?? [];
		for (const it of items) {
			if (!it.placements || untrack(() => it.id in placedElsewhere)) continue;
			placedElsewhere = { ...untrack(() => placedElsewhere), [it.id]: [] };
			api.get<{ placements: { key: string; count: number }[] }>(`/catalog/items/${encodeURIComponent(it.id)}/placements`)
				.then((r) => (placedElsewhere = { ...placedElsewhere, [it.id]: r.placements ?? [] }))
				.catch(() => {});
		}
	});
	/** per limited item: its limit, the placements in this chapter's documents and elsewhere (other chapters) */
	const placementCounts = $derived.by(() => {
		const out = new Map<string, { limit: number; here: number; others: { key: string; count: number }[] }>();
		const own = new Set(sliceDocs.map((d) => d.entity));
		const here = new Map<string, number>();
		for (const d of sliceDocs) if (!d.loading) for (const [id, n] of countPlacements(d.data[d.primary])) here.set(id, (here.get(id) ?? 0) + n);
		for (const it of itemCatalog.catalog?.items ?? []) {
			if (!it.placements) continue;
			out.set(it.id, { limit: it.placements, here: here.get(it.id) ?? 0, others: (placedElsewhere[it.id] ?? []).filter((p) => !own.has(p.key)) });
		}
		return out;
	});
	const placesText = (others: { key: string; count: number }[]) =>
		[m['creator.clip_limit_here'](), ...others.map((p) => `${fileLabel(p.key)} (${p.count})`)].join(', ');
	/** a paste that would place catalog items over their limit: the refusal (a toast); null = fine */
	const placementLimit = (adds: Map<string, number>): ClipResult | null => {
		for (const [id, n] of adds) {
			const c = placementCounts.get(id);
			if (!c) continue;
			const total = c.here + n + c.others.reduce((sum, p) => sum + p.count, 0);
			if (total > c.limit) {
				const item = itemCatalog.catalog?.items.find((i) => i.id === id);
				return { message: m['creator.clip_limit']({ item: itemName(item, id), limit: c.limit, count: total, where: placesText(c.others) }), error: true };
			}
		}
		return null;
	};
	useClipHandler(
		() => (view.section === 'slice' ? SLICE_REGION : null),
		// svelte-ignore state_referenced_locally
		sliceClipHandler({ view, target: sliceTarget, toon, chapterNumber, chapter, placementLimit })
	);
	// the slice list, the tree and the VN events: their copy / paste share the chapter, its box ids, labels, the limits
	$effect(() =>
		setClipContext({
			chapter,
			toon,
			chapterNumber,
			boxIds: chapterDialogIds,
			groupLabels: groupIds,
			placementLimit
		})
	);

	function onKeydown(event: KeyboardEvent) {
		const mod = event.ctrlKey || event.metaKey;
		const key = event.key.toLowerCase();
		// Ctrl+S saves at once; undo / redo only outside a field – in one, they are the browser's (lib/editor/editor-keys.ts)
		if (editorKeys(event, { save: () => void session.flush(), undo: undoLast, redo: redoLast })) return;
		if (isField(event.target) || view.section !== 'slice') return;
		// the keys act on the LIVE region only (DESIGN.md "Selection", BACKLOG 10.40): the canvas + its inspector. A
		// Delete in the slice list or the tree no longer removes the canvas's text box.
		if (!liveRegion.is(SLICE_REGION)) return;
		// one grid step (snap off: 1 px), Shift × 10
		const step = (view.grid || 1) * (event.shiftKey ? 10 : 1);
		const arrows: Record<string, [number, number]> = { arrowleft: [-step, 0], arrowright: [step, 0], arrowup: [0, -step], arrowdown: [0, step] };
		if (arrows[key] && view.selection) {
			event.preventDefault();
			canvas?.nudge(...arrows[key]);
		} else if ((key === 'delete' || key === 'backspace') && !mod) {
			// Delete / Backspace remove the selected text, item, sample area or event area (Ctrl+X = cut: the clipboard)
			if (!view.selection) return;
			event.preventDefault();
			removeSelected();
		} else if (key === 'escape') {
			view.selection = null;
		}
	}

</script>

<!-- the open group: its form (id, layout, end …) and its script – the group section, and the slice editor's Group tab
     (the strip editor, TEXT-ACROSS-SLICES.md §4.1: the group and its slices on one screen) -->
{#snippet groupBody()}
	{#if activeDoc}
		<GroupPanel doc={activeDoc} {groupPath} {toon} {chapterNumber} {groupLabels} last={view.groupIndex === groups.length - 1} idProblem={(from, to) => groupRenameProblem(groups, from, to)} onrename={renameGroup} />
		<h3 class="sub">{m['creator.vn']()}</h3>
		{#if activeGroup?.events}
			<VnEvents doc={activeDoc} eventsPath={[...groupPath, 'events']} {characters} {choices} dialogs={edits.boxChoices(activeGroup?.slices, [displayLocale, languages.source])} {groupLabels} markOf={(i) => markAt('groups', view.groupIndex, 'events', i)} />
		{:else}
			<p><button class="secondary" onclick={() => activeDoc?.edit((f) => { const g = getAt(f, groupPath); if (g) g.events = []; })}>+ {m['creator.vn_script_add']()}</button></p>
		{/if}
	{/if}
{/snippet}

<svelte:window onkeydown={onKeydown} />
<!-- right-click in a copy region: Copy · Cut · Paste · Paste settings (BACKLOG 10.39 step 8) -->
<ClipMenu />

{#snippet flowTab()}
	<FlowTab {...flowProps()} />
{/snippet}
{#snippet previewTab()}
	<PreviewTab {...readerProps()} />
{/snippet}

{#snippet tree()}
	<ChapterTree {view} {chapter} {groups} {groupName} {activeDoc} {groupPath} {chapterNumber} {markAt} />
{/snippet}

<div class="editor">


	{#if chapter.loading}
		<p class="pad">{m['creator.loading']()}</p>
	{:else if chapter.error}
		<p class="pad error">{chapter.error}</p>
	{:else}
		<DocNotices {session} docs={noticeDocs} />

		<!-- the slice canvas + its inspector = ONE selection region (live on load; the keys act on it) -->
		<div class="columns" use:region={view.section === 'slice' ? { name: SLICE_REGION, initial: true } : null}>
			<!-- Main -->
			<main class="main" class:fit={view.section === 'slice' && !!activeSlice}>
				{#if view.section === 'meta'}
					{#if legacyTexts}
						<div class="sc-bar is-note" data-testid="legacy-texts">
							<span>{m['creator.legacy_texts']({ count: legacyTexts })}</span>
							<button class="secondary" onclick={convertLegacyTexts} data-testid="convert-presets">{m['creator.legacy_texts_convert']()}</button>
						</div>
					{/if}
					<MetaPanel doc={chapter} {season} number={chapterNumber} {images} />
				{:else if view.section === 'audio'}
					<AudioPanel doc={chapter} {season} number={chapterNumber} />
				{:else if view.section === 'group'}
					{#if activeDoc && !activeDoc.loading}
						{@render groupBody()}
					{:else if activeDoc?.error}
						<p class="error">{activeDoc.error}</p>
					{:else}
						<p>{m['creator.loading']()}</p>
					{/if}
				{:else if view.section === 'slice' && activeDoc && activeSlice}
					<!-- the tool strip left of the canvas, the canvas fills the rest (UI-CONCEPTS E.4); the canvas is the group's strip –
					     every slice, the open one holds the selection (TEXT-ACROSS-SLICES.md §4) -->
					<div class="slice-room">
					<SliceToolbar {view} expressions={sliceHasExpressions(activeSlice)} />
					<div class="canvas-scroll" {@attach wheelZoom} data-testid="canvas-area">
						<StripCanvas
							bind:this={canvas}
							doc={activeDoc}
							{groupPath}
							active={view.sliceIndex}
							seams={view.seams}
							locale={displayLocale}
							grid={view.grid}
							tool={view.tool}
							layers={view.layers}
							selection={view.selection}
							live={liveRegion.is(SLICE_REGION)}
							zoom={view.zoom}
							pieceName={(p) => pieceLabel(p, choices?.registry, choices?.moods)}
							presets={collagePresets}
							onshown={(f) => (view.shownZoom = f)}
							onselect={(i, next) => {
								view.focusSlice(i);
								view.select(next);
							}}
							oncreate={(i, t, rect) => {
								view.focusSlice(i);
								CREATE[t](rect);
							}}
							oncentre={(i) => !view.selection && view.focusSlice(i)}
						/>
					</div>
					</div>
				{:else}
					<p class="sc-empty pad">{m['creator.select_slice']()}</p>
				{/if}
			</main>

		</div>
		{#if view.section === 'slice' && activeSlice}
			<ItemPickerSheet bind:open={view.pickingItem} title={m['creator.add_slice_item']()} onpick={(item) => addSliceItem(item.id)} />
		{/if}
	{/if}

</div>


{#snippet deskSliceNav()}
	{#if view.section === 'slice' && activeGroup?.slices?.length}
		<span class="slice-nav" role="group" aria-label={m['creator.slice_navigation']()}>
			<button class="ghost" onclick={() => view.selectSlice(view.sliceIndex - 1)} disabled={view.sliceIndex <= 0} title={m['creator.prev_slice']()} aria-label={m['creator.prev_slice']()} data-testid="prev-slice"><Icon name="chevron-left" /></button>
			<span class="slice-long">{m['creator.slice_of']({ index: view.sliceIndex + 1, count: activeGroup.slices.length })}</span><span class="slice-short">{view.sliceIndex + 1} / {activeGroup.slices.length}</span>
			<button class="ghost" onclick={() => view.selectSlice(view.sliceIndex + 1)} disabled={view.sliceIndex >= activeGroup.slices.length - 1} title={m['creator.next_slice']()} aria-label={m['creator.next_slice']()} data-testid="next-slice"><Icon name="chevron-right" /></button>
		</span>
	{/if}
{/snippet}

<!-- the slice's inspector: a panel tab, in the slice canvas' selection region (the keys act on both) -->
{#snippet inspectorTab()}
	<div class="inspector-tab" use:region={SLICE_REGION} data-testid="inspector-region">
		{@render deskSliceNav()}
		{#if activeDoc && activeSlice}
	<SliceInspector
		{view}
		groupTab={groupBody}
		{groupPath}
		boxOf={(i) => canvas?.drawnBox(i) ?? null}
		onpick={(i, next) => {
			view.focusSlice(i);
			view.select(next);
		}}
		doc={activeDoc}
		slice={activeSlice}
		{slicePath}
		group={activeGroup}
		{toon}
		{chapterNumber}
		seasonNumber={Number(season.get(season.primary, ['number'])) || 1}
		locale={displayLocale}
		{characters}
		{choices}
		presets={colorations}
		{groupLabels}
		chapterAudio={chapterData?.audio}
		{librarySample}
		{heldMusic}
		onaddtext={(preset) => addDialog(undefined, preset)}
		oncreatebox={createBoxForEvent}
		onremove={removeSelected}
		eventMark={(i) => markAt('groups', view.groupIndex, 'slices', view.sliceIndex, 'events', i)}
	/>
		{/if}
	</div>
{/snippet}

<style>
	/* DESIGN.md "Brutalist tools": the reader's type sizes, 2px ink frames, selected = inverted, ≥ 44px controls.
	   The stage runs in "fill" mode here: the editor is exactly the stage and scrolls its own columns. */
	.editor {
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow: hidden;
	}
	/* the bar's slice stepper (desktop): the count without its word below 1280 */
	.slice-short {
		display: none;
	}
	@media (max-width: 1279px) {
		.slice-long {
			display: none;
		}
		.slice-short {
			display: inline;
		}
	}
	.slice-nav {
		display: inline-flex;
		align-items: center;
		gap: var(--space-025);
		white-space: nowrap;
	}
	/* status bars: ink on paper with a thick state rule (no tinted fills); unsaved = inverted */
	.columns {
		position: relative;
		flex: 1;
		min-height: 0;
		display: grid;
		grid-template-columns: minmax(0, 1fr);
	}
	.inspector-tab {
		display: flex;
		flex-direction: column;
		min-height: 0;
	}
	.inspector-tab .slice-nav {
		padding: 0 var(--editor-pad);
	}
	/* a form is a sheet in the stage's middle; the slice canvas lies between the preset's panels (they float over it) */
	.main {
		overflow: auto;
		min-width: 0;
	}
	.main:not(.fit) {
		box-sizing: border-box;
		justify-self: center;
		width: min(60rem, 100% - 2rem);
		margin-bottom: var(--space-1);
		padding: var(--space-1) var(--space-1) var(--space-3);
		/* the form stands on the stage itself: no sheet behind it (RULES H11) */
	}
	.canvas-scroll {
		overflow: auto;
		padding: var(--space-1) 0 var(--space-15);
		/* two fingers zoom the slices (pinchZoom), not the whole page */
		touch-action: pan-x pan-y;
	}
	/* the slice editor (UI-CONCEPTS E.4): the tool strip left, the slice fills the rest – fit = the column's height (at most
	   2x the slice), zoomed = the area scrolls (owner 2026-10-03 + 2026-10-07) */
	.main.fit {
		display: flex;
		flex-direction: column;
		padding: var(--space-1) 27rem var(--space-1) 20rem;
	}
	.slice-room {
		flex: 1;
		display: flex;
		gap: var(--space-05);
		min-height: 12rem;
	}
	.main.fit .canvas-scroll {
		flex: 1;
		min-width: 0;
		padding: 0;
		container-type: size;
	}
	.sub {
		margin-top: var(--space-15);
	}
	.pad {
		padding: var(--space-1);
	}
	.error {
		color: var(--tool-stop);
	}
</style>
