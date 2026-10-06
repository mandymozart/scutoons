<script lang="ts">
	import type { VariantMedia } from '@scutoons/media';
	import { leaveGuard } from '$lib/editor/ask.ts';
	import { VIEWER_URL } from '$lib/config.ts';
	import { onMount, tick, untrack } from 'svelte';
	import { ActionBar, CloseIcon, confirmDialog, Icon, panelWidth } from '@scutoons/ui';
	import { beforeNavigate, replaceState } from '$app/navigation';
	import { page } from '$app/state';
	import { usePageNav } from '$lib/nav.svelte.ts';
	import { fileLabel, sceneLabel } from '$lib/labels.ts';
	import { DocSession } from '$lib/editor/doc-session.svelte.ts';
	import { publishLabel } from '$lib/editor/reviews.svelte.ts';
	import { editorKeys, isField } from '$lib/editor/editor-keys.ts';
	import DocNotices from '$lib/components/page/doc-notices.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { sceneDoc, toonDoc } from '$lib/editor/languages.ts';
	import { entities, getAt, deleteAt, Renderer, sceneNameOfRef, scriptLabels, type Path, type ToonLanguages } from '@scutoons/content';
	import { castEntries, castIndex, chapterSceneCharacters, characterIndex, characterName, expressionIndex, featuringSuggestions, REGULAR_PERSONA, toonStar, validateCast } from '@scutoons/content';
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
	import { validateFile, validateNeutral, chapterReach, scriptJumps, toonFacts, type ChapterReach, type FlowNode, type Problem, type ToonFacts } from '@scutoons/content';
	import { loadToonDocs } from '$lib/editor/toon-facts.ts';
	import type { ReachMark } from '$lib/components/chapter/reach-chip.svelte';
	import SliceCanvas from '$lib/components/slice/slice-canvas.svelte';
	import { legacyTextCount, legacyPlan, applyLegacyPlan, type TextPreset } from '$lib/editor/text-presets.ts';
	import { itemCatalog } from '$lib/editor/catalog.svelte.ts';
	import { ChapterView, type Section } from '$lib/editor/chapter-state.svelte.ts';
	import * as edits from '$lib/editor/slice-edit.ts';
	import ChapterTree from '$lib/components/chapter/chapter-tree.svelte';
	import SliceToolbar from '$lib/components/slice/slice-toolbar.svelte';
	import SliceInspector from '$lib/components/slice/slice-inspector.svelte';
	import { mapDoc as worldMapDoc } from '$lib/editor/map.ts';
	import { chapterSlices } from '@scutoons/content';
	import MetaPanel from '$lib/components/chapter/meta-panel.svelte';
	import FeaturingField from '$lib/components/fields/featuring-field.svelte';
	import RelatedItemsField from '$lib/components/fields/related-items-field.svelte';
	import GroupPanel from '$lib/components/chapter/group-panel.svelte';
	import StripPreview from '$lib/components/chapter/strip-preview.svelte';
	import AudioPanel from '$lib/components/chapter/audio-panel.svelte';
	import VnEvents from '$lib/components/events/vn-events.svelte';
	import { pagePanel } from '$lib/ask.svelte.ts';
	import { isAccepted } from '$lib/editor/accepted.ts';
	import ProblemsPanel from '$lib/components/page/problems-panel.svelte';
	import HistoryPanel from '$lib/components/page/history-panel.svelte';
	import FlowPanel from '$lib/components/flow/flow-panel.svelte';
	import FileStatus, { hasStatus } from '$lib/components/status/file-status.svelte';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';

	/** `languages`: the toon's source + languages (ToonIndex, loaded by the page) – every file is edited in these */
	let { toon, chapterNumber, languages }: { toon: string; chapterNumber: number; languages: ToonLanguages } = $props();

	const panelSize = panelWidth('sc-panel-width:chapter', 16); // normal = --tool-panel-width
	// the Flow panel's graph needs more room from the start (owner 2026-10-06: drags wider up to half the window too)
	const flowSize = panelWidth('sc-panel-width:flow', 28);
	// the slice inspector drags wider too (owner 2026-10-03): from its normal 22rem up to half the window
	let columnsEl = $state<HTMLElement>();
	const inspectorSize = panelWidth('sc-panel-width:slice-inspector', 22, () => columnsEl?.getBoundingClientRect().right ?? window.innerWidth);

	// ── Documents ───────────────────────────────────────────────────────────────────────────────────────
	// the page re-creates this component per chapter and toon languages ({#key}), so the initial props are the right ones
	// svelte-ignore state_referenced_locally
	const chapter = toonDoc(entities.chapter(toon, chapterNumber), languages);
	// the season holds the chapter's status (visibility, per locale): edited in the meta panel, saved with the chapter
	// svelte-ignore state_referenced_locally
	const season = toonDoc(entities.season(toon, 1), languages);
	let refs = $state<Record<string, ContentDoc>>({});
	// the season's cast + the character registry: the VN character fields pick from every character (the season's cast
	// first); picking someone who isn't cast adds them to the season's cast (saved / published with the chapter)
	/** the chapter's season (the editor edits season 1) */
	const SEASON = 1;
	// svelte-ignore state_referenced_locally
	const cast = castDoc(toon);
	const registry = registryDoc();
	const moods = moodsDoc();
	// the world map: a slice unlocks places (agents/MAP.md, edited here and in the map builder – one storage, the place's
	// `unlock`); saved / published with the chapter once edited here
	const worldMap = worldMapDoc();
	let mapTouched = $state(false);
	let toonInfos = $state<ToonInfo[]>([]);
	let castTouched = $state(false);
	let castNotice = $state<string | null>(null);
	const registryIndex = $derived(characterIndex(registryOf(registry)));
	const hasCast = $derived(!!castOf(cast) && !!registryOf(registry));
	const seasonCast = $derived(castEntries(castOf(cast), SEASON));
	const characters = $derived<{ name: string; label?: string }[]>(
		hasCast ? seasonCast.map((e) => ({ name: e.character, label: characterName(registryIndex.get(e.character), languages.source) || e.character })) : []
	);
	function ensureCast(character: string, persona: string) {
		const changed = addToSeasonCast(cast, SEASON, character, persona);
		if (!changed) return;
		castTouched = true;
		castNotice = m['creator.characters_cast_added']({ name: characterName(registryIndex.get(character), languages.source, persona) || character, season: m['creator.season']({ number: SEASON }) });
	}
	function undoCast() {
		cast.undo();
		castNotice = null;
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
					locale: languages.source,
					ensureCast
				}
	);
	let images = $state<string[]>([]);

	const chapterData = $derived(chapter.data[chapter.primary]);
	const groups = $derived<any[]>(chapterData?.groups ?? []);
	/** the toon's star (always in the cast, first in the suggested featuring) */
	const star = $derived(toonStar(registryOf(registry), toon));
	/** the chapter's groups with each scene document (source language) in place of its ref – what the VN plays */
	const sceneGroups = () => groups.map((g) => (g?.ref?.src ? (refs[g.ref.src]?.data?.[refs[g.ref.src].primary] ?? {}) : g));
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
		const all = sceneGroups();
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
		...Object.values(refs),
		...(castTouched || cast.dirty ? [cast] : []),
		...(mapTouched || worldMap.dirty ? [worldMap] : [])
	]);
	const anyDirty = $derived(allDocs.some((d) => d.dirty));
	/** the chapter's documents with slices (the chapter + its separate scenes) */
	const sliceDocs = $derived<ContentDoc[]>([chapter, ...Object.values(refs)]);
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

	async function ensureRef(src: string) {
		if (refs[src]) return refs[src];
		const doc = sceneDoc(toon, src, languages);
		refs[src] = doc;
		await doc.load();
		return doc;
	}

	onMount(async () => {
		await Promise.all([chapter.load(), season.load(), cast.load(), registry.load(), moods.load(), worldMap.load()]);
		loadToons(getLocale()).then((list) => (toonInfos = list));
		await Promise.all(groups.filter((g) => g?.ref?.src).map((g) => ensureRef(g.ref.src)));
		refreshImages();
		// the rest of the toon (reachability: labels something jumps to, variables something sets) – without it no
		// condition is judged and every labelled group counts as reachable
		loadToonDocs(toon, new Set(sliceDocs.map((d) => d.entity)))
			.then((docs) => (toonDocsFacts = toonFacts(docs)))
			.catch(() => (toonDocsFacts = null));
	});

	// ── Reachability (owner 2026-10-06, @scutoons/content reach.ts) ─────────────────────────────────────────────────
	let toonDocsFacts = $state<ToonFacts | null>(null);
	/** the toon's facts: the other documents (loaded once) + this chapter's own, live */
	const facts = $derived.by<ToonFacts | undefined>(() => {
		const rest = toonDocsFacts;
		if (!rest) return undefined;
		const live = toonFacts([...sliceDocs.map((d) => $state.snapshot(d.data[d.primary])), $state.snapshot(worldMap.data?.[worldMap.primary])]);
		return { variables: new Set([...rest.variables, ...live.variables]), labels: new Set([...rest.labels, ...live.labels]) };
	});
	/** the chapter's reachability for the labels in the tree / slice list / event lists – debounced while editing */
	let reach = $state<ChapterReach | null>(null);
	/** the groups `reach` was computed from (the Flow panel draws exactly what the chips say) */
	let reachGroups = $state.raw<any[] | null>(null);
	$effect(() => {
		const snapshot = $state.snapshot(sceneGroups());
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
		return out;
	});
	const markAt = (...path: (string | number)[]): ReachMark => marks.get(path.join('.')) ?? null;

	function refreshImages() {
		knownImages(toon, chapter.textLanguages, allDocs.map((d) => $state.snapshot(d.data))).then((list) => (images = list));
	}

	// ── Selection / view state: one object (lib/editor/chapter-state.svelte.ts) – the tree, the canvas, the inspector
	// and the keyboard write it; opening a group loads its separate scene file
	const view = new ChapterView((index) => {
		const src = groups[index]?.ref?.src;
		if (src) ensureRef(src);
	});
	// the canvas, the strip preview and the inspector show the top bar's TEXT language (default the source; owner
	// 2026-10-06: one language everywhere, the header's switch the only one)
	const displayLocale = $derived(textLanguage.current(languages.languages, languages.source));
	// snap + grid size are remembered on this device
	$effect(() => {
		void view.snap;
		void view.gridSize;
		view.savePrefs();
	});
	let panel = $state<'problems' | 'history' | 'flow' | null>(null);
	// with the Ask panel open too, the sidebar folds to its rail (the page keeps room)
	$effect(() => {
		pagePanel.open = panel !== null;
		return () => (pagePanel.open = false);
	});
	function togglePanel(next: 'problems' | 'history' | 'flow') {
		panel = panel === next ? null : next;
	}
	let canvas = $state<SliceCanvas | null>(null);
	/** save / publish / conflict / the notice (lib/editor/doc-session.svelte.ts – the same as the meta pages') */
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

	const currentGroup = $derived(groups[view.groupIndex]);
	const refSrc = $derived<string | null>(currentGroup?.ref?.src ?? null);
	const activeDoc = $derived<ContentDoc | null>(refSrc ? (refs[refSrc] ?? null) : chapter);
	/**
	 * undo / redo act on the file edited last (owner 2026-10-03: moving groups edits the chapter while a scene file is
	 * open) – the open group's file or the chapter; redo on the file that was undone last
	 */
	const undoDoc = $derived<ContentDoc | null>(
		[activeDoc, chapter].filter((d): d is ContentDoc => !!d?.canUndo).sort((a, b) => b.lastEditAt - a.lastEditAt)[0] ?? null
	);
	let undoneDoc = $state<ContentDoc | null>(null);
	const redoDoc = $derived<ContentDoc | null>(undoneDoc?.canRedo ? undoneDoc : activeDoc?.canRedo ? activeDoc : chapter.canRedo ? chapter : null);
	function undoLast() {
		const d = undoDoc;
		if (!d) return;
		d.undo();
		undoneDoc = d;
	}
	const redoLast = () => redoDoc?.redo();
	const groupPath = $derived<Path>(refSrc ? [] : ['groups', view.groupIndex]);
	const slicePath = $derived<Path>([...groupPath, 'slices', view.sliceIndex]);
	const activeGroup = $derived(activeDoc?.get(activeDoc.primary, groupPath));
	const activeSlice = $derived(view.sliceIndex >= 0 ? activeDoc?.get(activeDoc.primary, slicePath) : null);
	const sliceDialogs = $derived<any[]>(activeSlice?.dialog ?? []);
	const sliceItems = $derived<any[]>(activeSlice?.items ?? []);

	// ── Navigation: the tree is the sidebar, group + slice are the deepest crumbs, the URL mirrors the selection ──
	// (`?group=<label>&slice=<n>`, `?section=audio`; deep links and Alt+ArrowUp / crumbs work through the URL)
	// the page re-creates this component per chapter ({#key})
	// svelte-ignore state_referenced_locally
	const chapterHref = `/${toon}/chapter/${chapterNumber}`;
	const groupKey = (g: any, i: number) => g?.label ?? (g?.ref?.src ? sceneNameOfRef(g.ref.src) : null) ?? String(i + 1);
	/** a separate scene's own label (stored in the scene; owner 2026-10-03: renamed scene-13 still showed "Scene 14") */
	const refLabel = (src: string): string | undefined => {
		const doc = refs[src];
		const label = doc && !doc.loading ? doc.get(doc.primary, ['label']) : undefined;
		return typeof label === 'string' && label.trim() ? label : undefined;
	};
	const groupName = (g: any, i: number) =>
		g?.ref?.src ? (refLabel(g.ref.src) ?? sceneLabel(toon, g.ref.src)) : (g?.label ?? `${m['creator.groups']()} ${i + 1}`);
	function hrefFor(next: { section: Section; group?: number; slice?: number }) {
		const params = new URLSearchParams();
		if (next.section === 'audio') params.set('section', 'audio');
		if ((next.section === 'group' || next.section === 'slice') && next.group !== undefined) params.set('group', groupKey(groups[next.group], next.group));
		if (next.section === 'slice' && next.slice !== undefined && next.slice >= 0) params.set('slice', String(next.slice + 1));
		const query = params.toString();
		return query ? `${chapterHref}?${query}` : chapterHref;
	}
	usePageNav({
		sidebar: tree,
		actions: flowToggle,
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
				// the chapter's visibility: its entry in the season (null = none)
				seasonStatus: doc === chapter ? seasonStatusOf(season, languages.source, chapterNumber) : undefined,
				// linked groups: a ref group's `continues` lives in its scene
				resolveGroup: (g: any) => (g?.ref?.src ? refs[g.ref.src]?.data?.[refs[g.ref.src].primary] : g),
				// reachability: what the rest of the toon jumps to / sets
				facts,
				// placed items (and related items) are checked against the catalog once it is loaded
				items: catalogItems,
				itemRespawn
			};
			if (data == null) {
				if (doc.source[doc.primary] === 'missing') out.push({ level: 'error', file: doc.entity, locale: null, path: [], message: m['creator.missing']() });
				continue;
			}
			// chapter, scenes, season: one language-neutral document, checked once + its texts per language
			out.push(...(doc.texts ? validateNeutral(doc.kind, data, doc.texts, ctx) : validateFile(doc.kind, data, doc.primary, ctx)));
		}
		if (castTouched && castOf(cast)) out.push(...validateCast(castOf(cast), { registry: registryOf(registry), languages: languages.languages, file: entities.cast(toon) }));
		// suggestions from the VN usage (info): who the scenes use but the featuring / the season cast lacks
		if (!chapter.loading && hasCast) out.push(...featuringSuggestions(chapterData?.featuring, chapterSceneCharacters(sceneGroups()), castIndex(castOf(cast), SEASON), chapter.entity));
		const order = { error: 0, warning: 1, info: 2 };
		return out.sort((a, b) => order[a.level] - order[b.level]);
	});
	const errorCount = $derived(problems.filter((p) => p.level === 'error').length);
	// accepted warnings ("Looks fine") don't count
	const warningCount = $derived(problems.filter((p) => p.level === 'warning' && !isAccepted(allDocs, p)).length);

	function gotoProblem(p: Problem) {
		let path = p.path;
		if (p.file === season.entity) {
			view.section = 'meta';
			return;
		}
		if (p.file !== chapter.entity) {
			const gi = groups.findIndex((g) => g?.ref?.src === p.file);
			if (gi < 0) return;
			view.selectGroup(gi);
		} else if (path[0] === 'groups' && typeof path[1] === 'number') {
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
	/** the chapter's group labels: a jump's group and the random list (CUTOVER 10.27) */
	const groupLabels = $derived(groups.map((g, gi) => groupName(g, gi)).filter(Boolean));
	/** every text box id of the chapter and its scenes: a new box gets a fresh one (owner 2026-10-06: unique in the chapter) */
	const chapterDialogIds = () =>
		new Set<string>(sceneGroups().flatMap((g: any) => (g?.slices ?? []).flatMap((s: any) => (s?.dialog ?? []).map((d: any) => d?.id).filter((id: unknown) => typeof id === 'string'))));
	const sliceTarget = (): edits.SliceTarget | null =>
		activeDoc && view.sliceIndex >= 0 ? { doc: activeDoc, slicePath, chapterNumber, sliceIndex: view.sliceIndex, slice: activeSlice, taken: chapterDialogIds } : null;

	// linked groups (owner 2026-10-06): a group whose script can jump away and that continues – the next group needs a
	// label, else nothing leads back to it (a reminder with one click, next to the script)
	const nextGroup = $derived(sceneGroups()[view.groupIndex + 1]);
	const needsNextLabel = $derived(!!activeGroup?.continues && scriptJumps(activeGroup) && !!nextGroup && !nextGroup.label);
	function labelNextGroup() {
		const index = view.groupIndex + 1;
		const raw = groups[index];
		const taken = new Set(sceneGroups().map((g: any) => g?.label).filter(Boolean));
		let label = raw?.ref?.src ? sceneNameOfRef(raw.ref.src) : `scene-${index + 1}`;
		for (let n = 2; !label || taken.has(label); n++) label = `scene-${index + 1}-${n}`;
		if (raw?.ref?.src) refs[raw.ref.src]?.edit((file) => void (file.label = label));
		else chapter.edit((file) => void (file.groups[index].label = label));
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
		if (t && itemId) view.select(edits.addSliceItem(t, itemId, spot));
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
		else session.notice = 'The chapter has no sample library yet (Audio in the tree).';
	}
	function removeSelected() {
		const t = sliceTarget();
		if (!t || !view.selection) return;
		edits.removeSelected(t, view.selection);
		view.selection = null;
	}

	const previewLabel = $derived(view.section === 'group' || view.section === 'slice' ? (activeGroup?.label ?? '') : '');
	/**
	 * The chapter in the live reader with its draft preview on (`?preview=draft`, viewer lib/draft-preview.svelte.ts):
	 * the saved drafts, at the open group. The locale prefix picks the text language; the reader has de and en only.
	 */
	const readerLocales = $derived(chapter.textLanguages.filter((l) => l === 'de' || l === 'en'));
	const previewHref = (locale: string) =>
		`${VIEWER_URL}/${locale}/${toon}/chapter/${chapterNumber}?preview=draft${previewLabel ? `&label=${encodeURIComponent(previewLabel)}` : ''}`;
	/** unsaved changes would be missing in the reader: save the draft first (with the note dialog), then open it */
	async function openPreview(event: MouseEvent, locale: string) {
		if (!anyDirty) return;
		event.preventDefault();
		const save = await confirmDialog({
			title: m['creator.preview_unsaved_title'](),
			message: m['creator.preview_unsaved_message'](),
			confirmLabel: m['creator.preview_unsaved_save']()
		});
		if (!save) return;
		await session.saveDraft();
		if (anyDirty) return;
		// a popup blocker may refuse the window after the dialogs: then a hint (not 'noopener': that always returns null)
		const tab = window.open(previewHref(locale), '_blank');
		if (tab) tab.opener = null;
		else session.notice = m['creator.preview_saved_open']();
	}

	// ── Guards, autosave, keyboard ──────────────────────────────────────────────────────────────────────
	// unsaved changes: leaving asks in-page (preview links open in a new tab); lib/editor/ask.ts
	beforeNavigate(leaveGuard(() => anyDirty));

	$effect(() => {
		const docs = allDocs;
		for (const d of docs) void JSON.stringify(d.data);
		const timer = setTimeout(() => docs.forEach((d) => d.autosave()), 1000);
		return () => clearTimeout(timer);
	});

	function onKeydown(event: KeyboardEvent) {
		const mod = event.ctrlKey || event.metaKey;
		const key = event.key.toLowerCase();
		// Ctrl+S saves (no note dialog); undo / redo only outside a field – in one, they are the browser's (owner
		// 2026-10-03, COMPONENTS.md 4.4.1); lib/editor/editor-keys.ts
		if (editorKeys(event, { save: () => session.saveAll(), undo: undoLast, redo: redoLast })) return;
		if (isField(event.target) || view.section !== 'slice') return;
		// one grid step (snap off: 1 px), Shift × 10
		const step = (view.grid || 1) * (event.shiftKey ? 10 : 1);
		const arrows: Record<string, [number, number]> = { arrowleft: [-step, 0], arrowright: [step, 0], arrowup: [0, -step], arrowdown: [0, step] };
		if (arrows[key] && view.selection) {
			event.preventDefault();
			canvas?.nudge(...arrows[key]);
		} else if (((key === 'delete' || key === 'backspace') && !mod) || (mod && key === 'x')) {
			// Delete / Backspace / Ctrl+X remove the selected text, item, sample area or event area
			if (!view.selection) return;
			event.preventDefault();
			removeSelected();
		} else if (key === 'escape') {
			view.selection = null;
		}
	}

</script>

<svelte:window onkeydown={onKeydown} />

<!-- the Flow panel opens where the Ask panel does: a round button in the top bar (owner 2026-10-06) -->
{#snippet flowToggle()}
	<button type="button" class="sc-round" aria-pressed={panel === 'flow'} title={m['creator.flow_open']()} aria-label={m['creator.flow_open']()} onclick={() => togglePanel('flow')} data-testid="flow-toggle"><Icon name="git-merge" /></button>
{/snippet}

{#snippet tree()}
	<ChapterTree {view} {chapter} {groups} {groupName} {activeDoc} {groupPath} {chapterNumber} {markAt} resolveGroup={(g) => (g?.ref?.src ? refs[g.ref.src]?.data?.[refs[g.ref.src].primary] : g)} />
{/snippet}

<div class="editor" class:with-panel={panel !== null} style:--panel-width={panel === 'flow' ? flowSize.css : panelSize.css}>

	<!-- Bottom action bar (PLATFORM.md "Bottom action bar"): navigation + validation summary · tools · save / publish -->
	<!-- prev / next slice: in the bar's end from 768px (always at hand), in "More" on phones (the end holds only save) -->
	{#snippet sliceNav(where: 'start' | 'end')}
		{#if view.section === 'slice' && activeGroup?.slices?.length}
			<span class="slice-nav in-{where}" role="group" aria-label={m['creator.slice_navigation']()}>
				<button class="ghost" onclick={() => view.selectSlice(view.sliceIndex - 1)} disabled={view.sliceIndex <= 0} title={m['creator.prev_slice']()} aria-label={m['creator.prev_slice']()} data-testid={where === 'end' ? 'prev-slice' : undefined}><Icon name="chevron-left" /></button>
				<span>{m['creator.slice_of']({ index: view.sliceIndex + 1, count: activeGroup.slices.length })}</span>
				<button class="ghost" onclick={() => view.selectSlice(view.sliceIndex + 1)} disabled={view.sliceIndex >= activeGroup.slices.length - 1} title={m['creator.next_slice']()} aria-label={m['creator.next_slice']()} data-testid={where === 'end' ? 'next-slice' : undefined}><Icon name="chevron-right" /></button>
			</span>
		{/if}
	{/snippet}
	<!-- the reader's type size needs the room: below 2048px start + secondary sit in "More" (DESIGN.md "Brutalist tools") -->
	<ActionBar
		label={m['creator.page_actions']()}
		moreLabel={m['creator.more']()}
		breakpoint={2048}
	>
		{#snippet start()}
			{@render sliceNav('start')}
			<button class="ghost" class:active={panel === 'problems'} onclick={() => togglePanel('problems')} data-testid="problems-toggle">
				{m['creator.problems']()} <span class:bad={errorCount > 0}>{errorCount}</span>/<span>{warningCount}</span>
			</button>
		{/snippet}
		{#snippet secondary()}
			<button class="ghost" title="{m['creator.undo']()} (Ctrl+Z)" aria-label={m['creator.undo']()} disabled={!undoDoc} onclick={undoLast} data-testid="undo"><Icon name="rotate-ccw" /></button>
			<button class="ghost" title="{m['creator.redo']()} (Ctrl+Shift+Z)" aria-label={m['creator.redo']()} disabled={!redoDoc} onclick={redoLast} data-testid="redo"><Icon name="rotate-cw" /></button>
			<span class="preview">
				{m['creator.preview']()}:
				{#each readerLocales as l}
					<a href={previewHref(l)} target="_blank" rel="noopener" onclick={(e) => openPreview(e, l)} title={m['creator.preview_in_reader']({ locale: l })} data-testid="preview-{l}">{l}</a>
				{/each}
			</span>
			<button class="ghost" class:active={panel === 'history'} onclick={() => togglePanel('history')}>{m['creator.history']()}</button>
			<input class="message in-start" type="text" placeholder={m['creator.note_label']()} aria-label={m['creator.note_label']()} bind:value={session.message} />
		{/snippet}
		{#snippet end()}
			{@render sliceNav('end')}
			<input class="message in-end" type="text" placeholder={m['creator.note_label']()} aria-label={m['creator.note_label']()} bind:value={session.message} />
			<button onclick={() => session.saveDraft()} disabled={!anyDirty || anySaving} data-testid="save-draft">
				{anySaving ? m['creator.saving']() : m['creator.save_draft']()}{anyDirty ? ' •' : ''}
			</button>
			<button class="secondary" onclick={() => session.publishAll()} disabled={anySaving} data-testid="publish">{publishLabel(toon)}</button>
		{/snippet}
	</ActionBar>

	{#if chapter.loading}
		<p class="pad">{m['creator.loading']()}</p>
	{:else if chapter.error}
		<p class="pad error">{chapter.error}</p>
	{:else}
		{#if hasStatus(chapter) || (refSrc && refs[refSrc] && (view.section === 'group' || view.section === 'slice') && hasStatus(refs[refSrc])) || (!season.loading && (season.dirty || season.unpublished.length))}
		<div class="files" data-testid="file-status">
			<FileStatus doc={chapter} label={fileLabel(chapter.entity)} />
			<!-- the scene's own status only while one of its parts is open (owner 2026-10-03: "scene-14" next to Meta confused) -->
			{#if refSrc && refs[refSrc] && (view.section === 'group' || view.section === 'slice')}<FileStatus doc={refs[refSrc]} label={refLabel(refSrc) ?? sceneLabel(toon, refSrc)} />{/if}
			{#if !season.loading && (season.dirty || season.unpublished.length)}<FileStatus doc={season} label={fileLabel(season.entity)} />{/if}
		</div>
		{/if}
		<DocNotices
			{session}
			docs={allDocs}
			dirty={anyDirty && !chapter.saving}
		/>
		{#if castNotice}
			<div class="sc-bar is-note" data-testid="cast-notice">
				{castNotice}
				<button class="secondary" onclick={undoCast} data-testid="cast-undo">{m['creator.undo']()}</button>
				<button class="ghost" onclick={() => (castNotice = null)} aria-label={m['creator.close']()} title={m['creator.close']()}><CloseIcon /></button>
			</div>
		{/if}

		<div class="columns" class:has-inspector={view.section === 'slice'} bind:this={columnsEl} style:--inspector-width={inspectorSize.css}>
			<!-- Main -->
			<main class="main" class:fit={view.section === 'slice' && !!activeSlice}>
				{#if view.section === 'meta'}
					{#if legacyTexts}
						<div class="sc-bar is-note" data-testid="legacy-texts">
							<span>{m['creator.legacy_texts']({ count: legacyTexts })}</span>
							<button class="secondary" onclick={convertLegacyTexts} data-testid="convert-presets">{m['creator.legacy_texts_convert']()}</button>
						</div>
					{/if}
					<MetaPanel doc={chapter} {season} number={chapterNumber} {images}>
						{#snippet featuring()}
							{#if !cast.loading && !registry.loading}
								<FeaturingField
									doc={chapter}
									cast={seasonCast}
									registry={registryOf(registry)}
									{star}
									locale={languages.source}
									{sceneGroups}
									addToCast={(id) => ensureCast(id, REGULAR_PERSONA)}
								/>
							{/if}
							<!-- the chapter's related items (catalog ids, the same in every language); the reader does not show them yet -->
							<RelatedItemsField doc={chapter} hint={m['creator.related_items_hint_chapter']()} testid="chapter-related-items" />
						{/snippet}
					</MetaPanel>
				{:else if view.section === 'audio'}
					<AudioPanel doc={chapter} {season} number={chapterNumber} />
				{:else if view.section === 'group'}
					{#if activeDoc && !activeDoc.loading}
						<GroupPanel doc={activeDoc} {groupPath} {refSrc} {toon} {chapterNumber} {groupLabels} last={view.groupIndex === groups.length - 1} />
						{#if activeGroup?.renderer?.type === Renderer.HORIZONTAL && activeGroup.renderer.params?.fullWidth}
							<StripPreview doc={activeDoc} {groupPath} locale={displayLocale} onopen={(i) => view.selectSlice(i)} />
						{/if}
						<h3 class="sub">{m['creator.vn']()}</h3>
						{#if needsNextLabel}
							<div class="sc-bar is-note" data-testid="next-label-hint">
								<span>{m['creator.next_label_hint']()}</span>
								<button class="secondary" onclick={labelNextGroup} data-testid="next-label-add">{m['creator.next_label_add']()}</button>
							</div>
						{/if}
						{#if activeGroup?.events}
							<VnEvents doc={activeDoc} eventsPath={[...groupPath, 'events']} {characters} {choices} dialogs={edits.boxChoices(activeGroup?.slices, [displayLocale, languages.source])} {groupLabels} markOf={(i) => markAt('groups', view.groupIndex, 'events', i)} />
						{:else}
							<p class="hint">{m['creator.vn_no_script']()}
								<button class="secondary" onclick={() => activeDoc?.edit((f) => { const g = getAt(f, groupPath); if (g) g.events = []; })}>+ {m['creator.vn_script_add']()}</button></p>
						{/if}
					{:else if activeDoc?.error}
						<p class="error">{activeDoc.error}</p>
					{:else}
						<p>{m['creator.loading']()}</p>
					{/if}
				{:else if view.section === 'slice' && activeDoc && activeSlice}
					<SliceToolbar {view} />
					<div class="canvas-scroll">
						<SliceCanvas
							bind:this={canvas}
							doc={activeDoc}
							{slicePath}
							locale={displayLocale}
							grid={view.grid}
							tool={view.tool}
							layer={view.layer}
							selection={view.selection}
							onselect={(next) => view.select(next)}
							oncreate={(t, rect) => (t === 'text' ? addDialog(rect) : t === 'item' ? view.pickItem({ x: rect.x, y: rect.y }) : t === 'event' ? addSliceEvent(rect) : addSampleArea(rect))}
						/>
					</div>
				{:else}
					<p class="hint">{m['creator.select_slice']()}</p>
				{/if}
			</main>

			<!-- Inspector -->
			{#if view.section === 'slice' && activeDoc && activeSlice}
				<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
				<div class="inspector-resize" role="separator" aria-orientation="vertical" aria-label={m['creator.resize_panel']()} title={m['creator.resize_panel']()} tabindex="0" onpointerdown={inspectorSize.start} onkeydown={inspectorSize.key} ondblclick={inspectorSize.reset}></div>
				<SliceInspector
					{view}
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
					{groupLabels}
					chapterAudio={chapterData?.audio}
					{librarySample}
					{heldMusic}
					{worldMap}
					chapterSlices={chapterSlices(sceneGroups())}
					onmaptouch={() => (mapTouched = true)}
					onaddtext={(preset) => addDialog(undefined, preset)}
					onadditem={addSliceItem}
					oncreatebox={createBoxForEvent}
					onremove={removeSelected}
					eventMark={(i) => markAt('groups', view.groupIndex, 'slices', view.sliceIndex, 'events', i)}
				/>
			{/if}
		</div>
	{/if}

	{#if panel}
			{@const size = panel === 'flow' ? flowSize : panelSize}
			<!-- a focusable separator is a widget (WAI-ARIA window splitter): drag the panel wider (owner 2026-10-03) -->
			<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
			<div class="panel-resize" role="separator" aria-orientation="vertical" aria-label={m['creator.resize_panel']()} title={m['creator.resize_panel']()} tabindex="0" onpointerdown={size.start} onkeydown={size.key} ondblclick={size.reset}></div>
		<aside class="side-panel" class:is-flow={panel === 'flow'}>
			{#if panel === 'problems'}
				<ProblemsPanel {problems} docs={allDocs} home={chapter.entity} ongoto={gotoProblem} onclose={() => (panel = null)} />
			{:else if panel === 'history' && activeDoc}
				<HistoryPanel doc={view.section === 'group' || view.section === 'slice' ? activeDoc : chapter} onclose={() => (panel = null)} />
			{:else if panel === 'flow'}
				<FlowPanel
					groups={reachGroups}
					{reach}
					chapter={chapterNumber}
					current={view.section === 'group' || view.section === 'slice' ? view.groupIndex : -1}
					name={flowName}
					text={flowText}
					onopen={openFlowNode}
					onclose={() => (panel = null)}
				/>
			{/if}
		</aside>
	{/if}
</div>

<style>
	/* DESIGN.md "Brutalist tools": the reader's type sizes, 2px ink frames, selected = inverted, ≥ 44px controls.
	   The AppShell runs in "fill" mode here: the editor is exactly the content area and scrolls its own columns. */
	.editor {
		display: flex;
		flex-direction: column;
		height: 100%;
		overflow: hidden;
	}
	.files {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
		align-items: center;
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.slice-nav {
		display: inline-flex;
		align-items: center;
		gap: var(--space-025);
		white-space: nowrap;
	}
	.message {
		width: 10rem !important;
	}
	/* the phone bar holds only save / publish: slice nav and the note move into "More" */
	@media (max-width: 767px) {
		.in-end {
			display: none !important;
		}
		.message.in-start {
			width: 100% !important;
		}
	}
	@media (min-width: 768px) {
		.in-start {
			display: none !important;
		}
	}
	.preview {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		white-space: nowrap;
	}
	.preview a {
		color: var(--ink);
	}
	.bad {
		color: var(--error);
	}
	/* status bars: ink on paper with a thick state rule (no tinted fills); unsaved = inverted */
	.columns {
		position: relative;
		flex: 1;
		min-height: 0;
		display: grid;
		grid-template-columns: minmax(0, 1fr);
	}
	.columns.has-inspector {
		grid-template-columns: minmax(0, 1fr) var(--inspector-width, 22rem);
	}
	/* the inspector's left border drags it wider (@scutoons/ui panelWidth) */
	.inspector-resize {
		position: absolute;
		z-index: 5;
		top: 0;
		bottom: 0;
		right: calc(var(--inspector-width, 22rem) - 0.375rem);
		width: 0.75rem;
		cursor: col-resize;
		touch-action: none;
	}
	.inspector-resize:hover,
	.inspector-resize:focus-visible {
		background: linear-gradient(var(--ink), var(--ink)) center / var(--border-width-focus) 100% no-repeat;
		outline: none;
	}
	/* narrower: the inspector goes below the canvas (sidebar 12rem + inspector 22rem + a 390px slice need ≈ 1280px) */
	@media (max-width: 1279px) {
		.columns.has-inspector {
			grid-template-columns: minmax(0, 1fr);
			grid-template-rows: minmax(0, 1fr) minmax(0, 50%);
		}
		.inspector-resize {
			display: none;
		}
	}
	.main {
		overflow: auto;
		padding: var(--space-1) var(--space-1) var(--space-3);
		min-width: 0;
	}
	.canvas-scroll {
		overflow: auto;
		padding: var(--space-1) 0 var(--space-15);
	}
	/* the slice editor: tools on top, the slice fits the rest of the column's height – no scrolling unless the slice is
	   taller than the column even at its smallest (owner 2026-10-03) */
	.main.fit {
		display: flex;
		flex-direction: column;
		padding-bottom: var(--space-1);
	}
	.main.fit .canvas-scroll {
		flex: 1;
		min-height: 12rem;
		padding-bottom: 0;
		container-type: size;
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
	.sub {
		margin-top: var(--space-15);
	}
	.side-panel {
		position: fixed;
		top: var(--top);
		/* left of the Ask panel: Ask belongs to the shell, the page's panels to the page */
		right: var(--ask-inset, 0rem);
		/* ends at the bottom ActionBar (its border and last lines are not hidden behind it) */
		bottom: var(--action-bar-height);
		width: var(--panel-width);
		background: var(--paper);
		border-left: var(--border-width-regular) solid var(--ink);
		overflow: auto;
		padding: var(--space-1) var(--space-1) var(--space-3);
		z-index: 20;
		box-sizing: border-box;
	}
	/* the Flow panel: its graph fills the panel's height (it pans / zooms itself) */
	.side-panel.is-flow {
		overflow: hidden;
		padding-bottom: var(--space-1);
	}
	.pad {
		padding: var(--space-1);
	}
	.error {
		color: var(--error);
	}
	/* the page makes room for the side panel instead of hiding under it (like the meta pages); phones: it covers the page */
	@media (min-width: 768px) {
		.editor.with-panel {
			margin-right: var(--panel-width);
		}
	}
	/* the side panel's left edge drags it wider (@scutoons/ui panelWidth) */
	.panel-resize {
		position: fixed;
		z-index: 21;
		top: var(--top);
		bottom: var(--action-bar-height);
		right: calc(var(--ask-inset, 0rem) + var(--panel-width) - 0.375rem);
		width: 0.75rem;
		cursor: col-resize;
		touch-action: none;
	}
	.panel-resize:hover,
	.panel-resize:focus-visible {
		background: linear-gradient(var(--ink), var(--ink)) center / var(--border-width-focus) 100% no-repeat;
		outline: none;
	}
	@media (max-width: 767px) {
		.panel-resize {
			display: none;
		}
	}
</style>
