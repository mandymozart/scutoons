<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { ActionBar, CloseIcon, Icon } from '@scutoons/ui';
	import { beforeNavigate, replaceState } from '$app/navigation';
	import { page } from '$app/state';
	import { usePageNav } from '$lib/nav.svelte.ts';
	import { fileLabel, fileLabels, sceneLabel } from '$lib/labels.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { ContentDoc, SaveConflict } from '$lib/editor/doc.svelte.ts';
	import { sceneDoc, toonDoc } from '$lib/editor/languages.ts';
	import { entities, getAt, deleteAt, sceneNameOfRef, sceneRef, type Path, type ToonLanguages } from '@scutoons/content';
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
	import type { CharacterChoices } from './characters/character-ref-field.svelte';
	import { newDialog, newGroup, nextDialogId } from '$lib/editor/defaults.ts';
	import { knownImages } from '$lib/editor/images.ts';
	import { seasonStatusOf, syncChapter } from '$lib/editor/visibility.ts';
	import { validateFile, validateLocales, type Problem } from '@scutoons/content';
	import { Trigger, TriggerMode } from '@scutoons/content';
	import SliceCanvas, { type CanvasSelection, type Scope, type Tool } from './slice-canvas.svelte';
	import SliceList from './slice-list.svelte';
	import DialogInspector from './dialog-inspector.svelte';
	import ItemInspector from './item-inspector.svelte';
	import { itemCatalog, itemName, loadCatalog } from '$lib/editor/catalog.svelte.ts';
	import SliceMedia from './slice-media.svelte';
	import SliceUnlocks from './map/slice-unlocks.svelte';
	import { mapDoc as worldMapDoc } from '$lib/editor/map.ts';
	import { chapterSlices } from '@scutoons/content';
	import MetaPanel from './meta-panel.svelte';
	import FeaturingField from './featuring-field.svelte';
	import RelatedItemsField from './related-items-field.svelte';
	import GroupPanel from './group-panel.svelte';
	import AudioPanel from './audio-panel.svelte';
	import SliceAudio from './slice-audio.svelte';
	import VnEvents from './vn-events.svelte';
	import ProblemsPanel from './problems-panel.svelte';
	import HistoryPanel from './history-panel.svelte';
	import FileStatus from './file-status.svelte';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';

	/** `languages`: the toon's source + languages (ToonIndex, loaded by the page) – every file is edited in these */
	let { toon, chapterNumber, languages }: { toon: string; chapterNumber: number; languages: ToonLanguages } = $props();

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
	const allDocs = $derived<ContentDoc[]>([
		chapter,
		season,
		...Object.values(refs),
		...(castTouched || cast.dirty ? [cast] : []),
		...(mapTouched || worldMap.dirty ? [worldMap] : [])
	]);
	const anyDirty = $derived(allDocs.some((d) => d.dirty));
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
	});

	function refreshImages() {
		knownImages(toon, chapter.locales, allDocs.map((d) => $state.snapshot(d.data))).then((list) => (images = list));
	}

	// ── Selection / view state ──────────────────────────────────────────────────────────────────────────
	type Section = 'meta' | 'audio' | 'group' | 'slice';
	let section = $state<Section>('meta');
	let groupIndex = $state(0);
	let sliceIndex = $state(-1);
	let selection = $state<CanvasSelection | null>(null);
	let inspectorTab = $state<'text' | 'items' | 'slice' | 'audio' | 'events' | 'map'>('text');
	// svelte-ignore state_referenced_locally
	let displayLocale = $state<string>(textLanguage.chosen(languages.languages) ?? languages.source);
	// the canvas shows the top bar's TEXT language when one is chosen (the buttons above the canvas still switch it)
	$effect(() => {
		const chosen = textLanguage.chosen(languages.languages);
		if (chosen) displayLocale = chosen;
	});
	let grid = $state(0.5);
	let tool = $state<Tool>('text');
	let scope = $state<Scope>('all');
	let panel = $state<'problems' | 'history' | null>(null);
	function togglePanel(next: 'problems' | 'history') {
		panel = panel === next ? null : next;
	}
	let canvas = $state<SliceCanvas | null>(null);
	let notice = $state<string | null>(null);
	let conflict = $state<{ doc: ContentDoc; error: SaveConflict } | null>(null);
	let saveMessage = $state('');

	const currentGroup = $derived(groups[groupIndex]);
	const refSrc = $derived<string | null>(currentGroup?.ref?.src ?? null);
	const activeDoc = $derived<ContentDoc | null>(refSrc ? (refs[refSrc] ?? null) : chapter);
	const groupPath = $derived<Path>(refSrc ? [] : ['groups', groupIndex]);
	const slicePath = $derived<Path>([...groupPath, 'slices', sliceIndex]);
	const activeGroup = $derived(activeDoc?.get(activeDoc.primary, groupPath));
	const activeSlice = $derived(sliceIndex >= 0 ? activeDoc?.get(activeDoc.primary, slicePath) : null);
	const sliceDialogs = $derived<any[]>(activeSlice?.dialog ?? []);
	const sliceItems = $derived<any[]>(activeSlice?.items ?? []);

	function selectGroup(index: number) {
		groupIndex = index;
		sliceIndex = -1;
		selection = null;
		section = 'group';
		const src = groups[index]?.ref?.src;
		if (src) ensureRef(src);
	}

	function selectSlice(index: number) {
		sliceIndex = index;
		selection = null;
		section = 'slice';
	}

	function onCanvasSelect(next: CanvasSelection | null) {
		selection = next;
		if (next?.kind === 'dialog') inspectorTab = 'text';
		else if (next?.kind === 'item') inspectorTab = 'items';
		else if (next?.kind === 'sample') inspectorTab = 'audio';
		else if (next?.kind === 'event') inspectorTab = 'events';
	}

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
		fill: true,
		extra: () => {
			if (section !== 'group' && section !== 'slice') return [];
			const out = [{ href: hrefFor({ section: 'group', group: groupIndex }), label: groupName(groups[groupIndex], groupIndex) }];
			if (section === 'slice' && sliceIndex >= 0) out.push({ href: hrefFor({ section: 'slice', group: groupIndex, slice: sliceIndex }), label: m['creator.slice_n']({ number: sliceIndex + 1 }) });
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
					if (gi !== groupIndex || (section !== 'group' && section !== 'slice')) selectGroup(gi);
					if (wantSlice > 0) {
						if (section !== 'slice' || sliceIndex !== wantSlice - 1) selectSlice(wantSlice - 1);
					} else if (section === 'slice') {
						selectGroup(gi);
					}
				}
			} else if (wantSection === 'audio') {
				section = 'audio';
			} else if (section !== 'meta') {
				section = 'meta';
			}
			urlApplied = true;
		});
	});
	$effect(() => {
		// selection → URL (shareable; a crumb goes back to the same place)
		const href = hrefFor({ section, group: groupIndex, slice: sliceIndex });
		if (!urlApplied) return;
		untrack(() => {
			if (href !== page.url.pathname + page.url.search) {
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
		for (const doc of allDocs) {
			if (doc.loading) continue;
			for (const l of doc.locales) {
				if (doc.source[l] === 'missing' && doc.data[l] == null) {
					out.push({ level: l === doc.primary ? 'error' : 'warning', file: doc.entity, locale: l, path: [], message: `${m['creator.missing']()}: ${l}` });
					continue;
				}
				out.push(
					...validateFile(doc.kind, doc.data[l], l, {
						file: doc.entity,
						audio: chapterData?.audio,
						characters: !hasCast && names.length ? names : undefined,
						cast: hasCast ? castIndex(castOf(cast), SEASON) : undefined,
						expressions,
						moods: moodIds,
						seasonStatus: doc === chapter ? seasonStatusOf(season, l, chapterNumber) : undefined,
						// placed items (and related items) are checked against the catalog once it is loaded
						items: itemCatalog.catalog ? itemCatalog.catalog.items.map((i) => i.id) : undefined
					})
				);
			}
			out.push(...validateLocales(doc.kind, doc.entity, doc.data, doc.primary));
		}
		if (castTouched && castOf(cast)) out.push(...validateCast(castOf(cast), { registry: registryOf(registry), languages: languages.languages, file: entities.cast(toon) }));
		// suggestions from the VN usage (info): who the scenes use but the featuring / the season cast lacks
		if (!chapter.loading && hasCast) out.push(...featuringSuggestions(chapterData?.featuring, chapterSceneCharacters(sceneGroups()), castIndex(castOf(cast), SEASON), chapter.entity));
		const order = { error: 0, warning: 1, info: 2 };
		return out.sort((a, b) => order[a.level] - order[b.level]);
	});
	const errorCount = $derived(problems.filter((p) => p.level === 'error').length);
	const warningCount = $derived(problems.filter((p) => p.level === 'warning').length);

	function gotoProblem(p: Problem) {
		let path = p.path;
		if (p.file === season.entity) {
			section = 'meta';
			return;
		}
		if (p.file !== chapter.entity) {
			const gi = groups.findIndex((g) => g?.ref?.src === p.file);
			if (gi < 0) return;
			selectGroup(gi);
		} else if (path[0] === 'groups' && typeof path[1] === 'number') {
			selectGroup(path[1]);
			path = path.slice(2);
		} else {
			section = path[0] === 'audio' ? 'audio' : 'meta';
			return;
		}
		if (p.locale) displayLocale = p.locale;
		if (path[0] === 'slices' && typeof path[1] === 'number') {
			selectSlice(path[1]);
			const rest = path.slice(2);
			if (rest[0] === 'dialog' && typeof rest[1] === 'number') onCanvasSelect({ kind: 'dialog', index: rest[1] });
			else if (rest[0] === 'items' && typeof rest[1] === 'number') onCanvasSelect({ kind: 'item', index: rest[1] });
			else if (rest[0] === 'audio') inspectorTab = 'audio';
			else if (rest[0] === 'events') inspectorTab = 'events';
			else inspectorTab = 'slice';
		}
	}

	// ── Editing actions ─────────────────────────────────────────────────────────────────────────────────
	function addDialog(rect = { x: 10, y: 10, width: 80, height: 12 }) {
		const doc = activeDoc;
		if (!doc || sliceIndex < 0) return;
		// ids follow the image number like chapter 1 (s1c1-<slice>-<n>), else the position in the group
		const imageNo = Number(String(activeSlice?.image ?? '').match(/(\d+)\.\w+$/)?.[1]);
		const id = nextDialogId(chapterNumber, (imageNo || sliceIndex + 1) - 1, sliceDialogs);
		doc.structural((file) => {
			const slice = getAt(file, slicePath);
			if (!slice) return;
			slice.dialog ??= [];
			slice.dialog.push(newDialog(id, { ...rect }));
		});
		onCanvasSelect({ kind: 'dialog', index: sliceDialogs.length - 1 });
		requestAnimationFrame(() => document.querySelector<HTMLTextAreaElement>(`[data-testid="text-${doc.primary}"]`)?.focus());
	}

	/** places a catalog item in the middle of the slice (a quarter of its width; move / rotate / scale on the canvas) */
	function addSliceItem(itemId: string) {
		const doc = activeDoc;
		if (!doc || sliceIndex < 0 || !itemId) return;
		const taken = new Set(sliceItems.map((it) => it?.id));
		let n = 1;
		while (taken.has(`${itemId}-${n}`)) n++;
		doc.structural((file) => {
			const slice = getAt(file, slicePath);
			if (!slice) return;
			slice.items ??= [];
			slice.items.push({ id: `${itemId}-${n}`, item: itemId, rect: { x: 37.5, y: 40, width: 25 } });
		});
		onCanvasSelect({ kind: 'item', index: sliceItems.length - 1 });
	}

	function addSampleArea(rect: { x: number; y: number; width: number; height: number }) {
		const doc = activeDoc;
		if (!doc) return;
		const library = chapterData?.audio?.sampleLibrary ?? [];
		if (!library.length) {
			notice = 'The chapter has no sample library yet (Audio in the tree).';
			return;
		}
		doc.structural((file) => {
			const slice = getAt(file, slicePath);
			if (!slice) return;
			slice.audio ??= {};
			slice.audio.audio ??= {};
			slice.audio.audio.samples ??= [];
			slice.audio.audio.samples.push({ id: library[0].id, trigger: { type: Trigger.CLICK, mode: TriggerMode.POLY, rect: { ...rect } }, volume: 0.6 });
		});
		onCanvasSelect({ kind: 'sample', index: (activeSlice?.audio?.audio?.samples?.length ?? 1) - 1 });
	}

	function removeSelected() {
		const doc = activeDoc;
		if (!doc || !selection) return;
		const { kind, index } = selection;
		const listPath =
			kind === 'dialog'
				? [...slicePath, 'dialog']
				: kind === 'item'
					? [...slicePath, 'items']
					: kind === 'sample'
						? [...slicePath, 'audio', 'audio', 'samples']
						: [...slicePath, 'events'];
		doc.structural((file) => {
			const list = getAt(file, listPath);
			if (!Array.isArray(list)) return;
			list.splice(index, 1);
			if (!list.length) deleteAt(file, listPath, slicePath.length);
		});
		selection = null;
	}

	function addGroup(ref: boolean) {
		const label = `scene-${groups.length + 1}`;
		if (ref) {
			// the scene's name is its key (stored with the chapter: chapter<N>/<name>)
			const name = prompt('Scene name', label)?.trim();
			if (!name) return;
			if (!/^[a-z0-9_-]+$/.test(name)) {
				notice = 'A scene name uses a-z, 0-9, - and _ (e.g. scene-21).';
				return;
			}
			const src = sceneRef(chapterNumber, name);
			const doc = sceneDoc(toon, src, languages);
			doc.create(() => newGroup(sceneNameOfRef(src) ?? name));
			refs[src] = doc;
			chapter.structural((file) => file.groups.push({ ref: { src } }));
		} else {
			chapter.structural((file) => file.groups.push(newGroup(label)));
		}
		selectGroup(groups.length - 1);
	}

	function moveGroup(index: number, by: number) {
		const to = index + by;
		if (to < 0 || to >= groups.length) return;
		chapter.structural((file) => {
			const [g] = file.groups.splice(index, 1);
			file.groups.splice(to, 0, g);
		});
		groupIndex = to;
	}

	function removeGroup(index: number) {
		const g = groups[index];
		if (!confirm(m['creator.remove_confirm']({ what: `group ${g?.label ?? g?.ref?.src ?? index + 1}` }))) return;
		chapter.structural((file) => file.groups.splice(index, 1));
		groupIndex = Math.max(0, index - 1);
		section = 'meta';
	}

	function moveSlice(by: number) {
		const doc = activeDoc;
		const to = sliceIndex + by;
		const list = activeGroup?.slices ?? [];
		if (!doc || to < 0 || to >= list.length) return;
		doc.structural((file) => {
			const slices = getAt(file, [...groupPath, 'slices']);
			const [s] = slices.splice(sliceIndex, 1);
			slices.splice(to, 0, s);
		});
		sliceIndex = to;
	}

	// ── Save / publish ──────────────────────────────────────────────────────────────────────────────────
	async function saveAll(force = false) {
		notice = null;
		conflict = null;
		// the chapter file's (deprecated) status mirrors the season, its old `published` flag goes
		syncChapter(chapter, season, chapterNumber);
		// a cast saved from here keeps the toon's star in every season
		if (castTouched || cast.dirty) castWithStar(cast, registry, toon, [SEASON]);
		for (const doc of allDocs) {
			if (!doc.dirty) continue;
			try {
				await doc.save(saveMessage.trim() || undefined, force);
			} catch (e) {
				if (e instanceof SaveConflict) conflict = { doc, error: e };
				else notice = `${fileLabel(doc.entity)}: ${e instanceof Error ? e.message : String(e)}`;
				return false;
			}
		}
		saveMessage = '';
		notice = m['creator.saved']();
		refreshImages();
		return true;
	}

	async function publishAll() {
		syncChapter(chapter, season, chapterNumber);
		const docs = allDocs.filter((d) => d.dirty || d.unpublished.length || d.stored.length);
		const files = fileLabels(docs.map((d) => d.entity));
		if (errorCount && !confirm(`${errorCount} errors in the problem list. Publish anyway?`)) return;
		if (!confirm(m['creator.publish_confirm']({ file: files, locales: chapter.locales.join(', ') }))) return;
		if (!(await saveAll())) return;
		try {
			for (const doc of allDocs) if (doc.stored.length && doc.unpublished.length) await doc.publish();
			notice = m['creator.published']();
		} catch (e) {
			notice = e instanceof Error ? e.message : String(e);
		}
	}

	async function resolveConflict(action: 'reload' | 'overwrite') {
		if (!conflict) return;
		const { doc } = conflict;
		conflict = null;
		if (action === 'reload') await doc.load();
		else await saveAll(true);
	}

	const previewLabel = $derived(section === 'group' || section === 'slice' ? (activeGroup?.label ?? '') : '');
	/** the creator's own draft preview (saved drafts rendered with @scutoons/reader), in a new tab */
	const previewHref = (locale: string) =>
		`/${toon}/chapter/${chapterNumber}/preview?locale=${locale}${previewLabel ? `&label=${encodeURIComponent(previewLabel)}` : ''}`;

	// ── Guards, autosave, keyboard ──────────────────────────────────────────────────────────────────────
	beforeNavigate((navigation) => {
		if (!anyDirty) return;
		if (navigation.type === 'leave') {
			navigation.cancel();
			return;
		}
		// preview links open in a new tab; everything else asks
		if (!confirm(m['creator.leave_confirm']())) navigation.cancel();
	});

	$effect(() => {
		const docs = allDocs;
		for (const d of docs) void JSON.stringify(d.data);
		const timer = setTimeout(() => docs.forEach((d) => d.autosave()), 1000);
		return () => clearTimeout(timer);
	});

	function isField(target: EventTarget | null) {
		const el = target as HTMLElement | null;
		return !!el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT' || el.isContentEditable);
	}

	function onKeydown(event: KeyboardEvent) {
		const mod = event.ctrlKey || event.metaKey;
		const key = event.key.toLowerCase();
		if (mod && key === 'z') {
			event.preventDefault();
			if (event.shiftKey) activeDoc?.redo();
			else activeDoc?.undo();
			return;
		}
		if (mod && key === 'y') {
			event.preventDefault();
			activeDoc?.redo();
			return;
		}
		if (mod && key === 's') {
			event.preventDefault();
			saveAll();
			return;
		}
		if (isField(event.target) || section !== 'slice') return;
		const step = (grid || 0.1) * (event.shiftKey ? 10 : 1);
		const arrows: Record<string, [number, number]> = { arrowleft: [-step, 0], arrowright: [step, 0], arrowup: [0, -step], arrowdown: [0, step] };
		if (arrows[key] && selection) {
			event.preventDefault();
			canvas?.nudge(...arrows[key]);
		} else if ((key === 'delete' || key === 'backspace') && selection) {
			event.preventDefault();
			removeSelected();
		} else if (key === 'escape') {
			selection = null;
		}
	}

	const recoverables = $derived(allDocs.filter((d) => d.recoverable));
	const missingLocales = $derived(
		allDocs.flatMap((d) => d.locales.filter((l) => l !== d.primary && d.data[l] == null && !d.loading && d.data[d.primary]).map((l) => ({ doc: d, locale: l })))
	);
</script>

<svelte:window onkeydown={onKeydown} />

{#snippet tree()}
	<!-- the chapter's children in the sidebar: meta, audio, groups → slices (Alt+ArrowDown opens the first child) -->
	<div class="tree" data-testid="chapter-tree">
		<button class="node sc-nav-item" class:is-current={section === 'meta'} onclick={() => (section = 'meta')} data-testid="tree-meta">{m['creator.meta']()}</button>
		<button class="node sc-nav-item" class:is-current={section === 'audio'} onclick={() => (section = 'audio')} data-testid="tree-audio">{m['creator.audio']()}</button>
		<div class="sc-nav-head">{m['creator.groups']()}</div>
		{#each groups as g, gi (gi)}
			{@const open = gi === groupIndex && (section === 'group' || section === 'slice')}
			<div class="group" class:open>
				<div class="group-row">
					<button
						class="node sc-nav-item"
						class:is-current={section === 'group' && gi === groupIndex}
						onclick={() => selectGroup(gi)}
						data-nav-child={section === 'meta' || section === 'audio' ? '' : undefined}
						data-testid="group-{gi}"
					>
						{groupName(g, gi)}
						<span class="hint">{g?.ref ? '' : `${g?.slices?.length ?? 0}${g?.events ? ' · VN' : ''}`}</span>
					</button>
					<span class="tools">
						<button class="ghost" onclick={() => moveGroup(gi, -1)} disabled={gi === 0} aria-label={m['creator.move_up']()} title={m['creator.move_up']()}><Icon name="arrow-up" /></button>
						<button class="ghost" onclick={() => moveGroup(gi, 1)} disabled={gi === groups.length - 1} aria-label={m['creator.move_down']()} title={m['creator.move_down']()}><Icon name="arrow-down" /></button>
						<button class="ghost" onclick={() => removeGroup(gi)} aria-label={m['creator.remove']()} title={m['creator.remove']()}><CloseIcon /></button>
					</span>
				</div>
				{#if open}
					{#if activeDoc && !activeDoc.loading}
						<div class="slices" data-child-slices={section === 'group' ? '' : undefined}>
							<SliceList doc={activeDoc} {groupPath} {chapterNumber} selected={section === 'slice' ? sliceIndex : -1} onselect={selectSlice} />
						</div>
					{:else}
						<p class="hint pad">{m['creator.loading']()}</p>
					{/if}
				{/if}
			</div>
		{/each}
		<div class="tree-add">
			<button class="secondary" title={m['creator.add_group']()} onclick={() => addGroup(false)}><span class="one-line">{m['creator.add_group']()}</span></button>
			<button class="secondary" title={m['creator.add_ref_group']()} onclick={() => addGroup(true)}><span class="one-line">{m['creator.add_ref_group']()}</span></button>
		</div>
	</div>
{/snippet}

<div class="editor" class:with-panel={panel !== null}>

	<!-- Bottom action bar (PLATFORM.md "Bottom action bar"): navigation + validation summary · tools · save / publish -->
	<!-- prev / next slice: in the bar's end from 768px (always at hand), in "More" on phones (the end holds only save) -->
	{#snippet sliceNav(where: 'start' | 'end')}
		{#if section === 'slice' && activeGroup?.slices?.length}
			<span class="slice-nav in-{where}" role="group" aria-label={m['creator.slice_navigation']()}>
				<button class="ghost" onclick={() => selectSlice(sliceIndex - 1)} disabled={sliceIndex <= 0} title={m['creator.prev_slice']()} aria-label={m['creator.prev_slice']()} data-testid={where === 'end' ? 'prev-slice' : undefined}><Icon name="chevron-left" /></button>
				<span>{m['creator.slice_of']({ index: sliceIndex + 1, count: activeGroup.slices.length })}</span>
				<button class="ghost" onclick={() => selectSlice(sliceIndex + 1)} disabled={sliceIndex >= activeGroup.slices.length - 1} title={m['creator.next_slice']()} aria-label={m['creator.next_slice']()} data-testid={where === 'end' ? 'next-slice' : undefined}><Icon name="chevron-right" /></button>
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
			<button class="ghost" title="{m['creator.undo']()} (Ctrl+Z)" aria-label={m['creator.undo']()} disabled={!activeDoc?.canUndo} onclick={() => activeDoc?.undo()} data-testid="undo"><Icon name="rotate-ccw" /></button>
			<button class="ghost" title="{m['creator.redo']()} (Ctrl+Shift+Z)" aria-label={m['creator.redo']()} disabled={!activeDoc?.canRedo} onclick={() => activeDoc?.redo()} data-testid="redo"><Icon name="rotate-cw" /></button>
			<span class="preview">
				{m['creator.preview']()}:
				{#each chapter.locales as l}
					<a href={previewHref(l)} target="_blank" rel="noopener" title="Draft preview in {l} (saved drafts)" data-testid="preview-{l}">{l}</a>
				{/each}
			</span>
			<button class="ghost" class:active={panel === 'history'} onclick={() => togglePanel('history')}>{m['creator.history']()}</button>
			<input class="message in-start" type="text" placeholder="note (optional)" aria-label="note (optional)" bind:value={saveMessage} />
		{/snippet}
		{#snippet end()}
			{@render sliceNav('end')}
			<input class="message in-end" type="text" placeholder="note (optional)" aria-label="note (optional)" bind:value={saveMessage} />
			<button onclick={() => saveAll()} disabled={!anyDirty || anySaving} data-testid="save-draft">
				{anySaving ? m['creator.saving']() : m['creator.save_draft']()}{anyDirty ? ' •' : ''}
			</button>
			<button class="secondary" onclick={publishAll} disabled={anySaving} data-testid="publish">{m['creator.publish']()}</button>
		{/snippet}
	</ActionBar>

	{#if chapter.loading}
		<p class="pad">{m['creator.loading']()}</p>
	{:else if chapter.error}
		<p class="pad error">{chapter.error}</p>
	{:else}
		<div class="files" data-testid="file-status">
			<FileStatus doc={chapter} label={fileLabel(chapter.entity)} />
			{#if refSrc && refs[refSrc]}<FileStatus doc={refs[refSrc]} label={refLabel(refSrc) ?? sceneLabel(toon, refSrc)} />{/if}
			{#if !season.loading && (season.dirty || season.unpublished.length)}<FileStatus doc={season} label={fileLabel(season.entity)} />{/if}
		</div>
		{#if anyDirty && !chapter.saving}<div class="bar dirty-bar">{m['creator.unsaved']()}{#if activeDoc?.dirty && section === 'slice'} · {activeDoc.dirtyLocales.join(', ')}{/if}</div>{/if}
		{#each recoverables as d (d.entity)}
			<div class="bar warn-bar">
				{fileLabel(d.entity)}: {m['creator.recover_title']({ time: new Date(d.recoverable!.at).toLocaleString() })}
				{#if d.recoverableIsStale}<strong class="sc-emphasis">{m['creator.recover_stale']()}</strong>{/if}
				<button class="secondary" onclick={() => d.recover()}>{m['creator.recover']()}</button>
				<button class="ghost" onclick={() => d.clearAutosave()}>{m['creator.discard']()}</button>
			</div>
		{/each}
		{#each missingLocales as { doc, locale } (doc.entity + locale)}
			<div class="bar warn-bar">
				{fileLabel(doc.entity)}: {locale} {m['creator.missing']()}
				<button class="secondary" onclick={() => doc.createLocale(locale, (f) => (f && doc.kind === 'chapter' ? { ...f, language: locale } : f))}>
					{m['creator.create_locale']({ locale, source: doc.primary })}
				</button>
			</div>
		{/each}
		{#if conflict}
			<div class="bar error-bar">
				{m['creator.conflict']({ locale: conflict.error.locale, file: fileLabel(conflict.doc.entity) })}
				<button class="secondary" onclick={() => resolveConflict('reload')}>{m['creator.reload']()}</button>
				<button class="secondary" onclick={() => resolveConflict('overwrite')}>{m['creator.overwrite']()}</button>
			</div>
		{/if}
		{#if notice}<div class="bar note-bar">{notice} <button class="ghost" onclick={() => (notice = null)} aria-label={m['creator.info_close']()} title={m['creator.info_close']()}><CloseIcon /></button></div>{/if}
		{#if castNotice}
			<div class="bar note-bar" data-testid="cast-notice">
				{castNotice}
				<button class="secondary" onclick={undoCast} data-testid="cast-undo">{m['creator.undo']()}</button>
				<button class="ghost" onclick={() => (castNotice = null)} aria-label={m['creator.info_close']()} title={m['creator.info_close']()}><CloseIcon /></button>
			</div>
		{/if}

		<div class="columns" class:has-inspector={section === 'slice'}>
			<!-- Main -->
			<main class="main">
				{#if section === 'meta'}
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
				{:else if section === 'audio'}
					<AudioPanel doc={chapter} />
				{:else if section === 'group'}
					{#if activeDoc && !activeDoc.loading}
						<GroupPanel doc={activeDoc} {groupPath} {refSrc} {toon} />
						<h3 class="sub">{m['creator.vn']()} (group.events)</h3>
						{#if activeGroup?.events}
							<VnEvents doc={activeDoc} eventsPath={[...groupPath, 'events']} {characters} {choices} background={activeGroup?.slices?.[0]?.image ?? ''} />
						{:else}
							<p class="hint">No VN script in this group.
								<button class="secondary" onclick={() => activeDoc?.structural((f) => { const g = getAt(f, groupPath); if (g) g.events = []; })}>+ VN script</button></p>
						{/if}
						<p class="hint">{m['creator.select_slice']()}</p>
					{:else if activeDoc?.error}
						<p class="error">{activeDoc.error}</p>
					{:else}
						<p>{m['creator.loading']()}</p>
					{/if}
				{:else if section === 'slice' && activeDoc && activeSlice}
					<div class="canvas-tools">
						<span class="seg" role="group" aria-label="locale">
							{#each activeDoc.present as l}
								<button class="ghost" class:active={displayLocale === l} onclick={() => (displayLocale = l)} data-testid="show-{l}">{l}</button>
							{/each}
						</span>
						<label class="inline">grid
							<select bind:value={grid}>
								<option value={0}>off</option>
								<option value={0.5}>0.5%</option>
								<option value={1}>1%</option>
								<option value={2.5}>2.5%</option>
								<option value={5}>5%</option>
							</select>
						</label>
						<span class="seg" role="group" aria-label="tool">
							<button class="ghost" class:active={tool === 'text'} onclick={() => (tool = 'text')}><Icon name="square" /> text box</button>
							<button class="ghost" class:active={tool === 'sample'} onclick={() => (tool = 'sample')}><Icon name="music" /> sample area</button>
						</span>
						<label class="inline" title="Geometry edits on the canvas and in the fields">geometry
							<select bind:value={scope}>
								<option value="all">all locales</option>
								<option value="locale">only {displayLocale}</option>
							</select>
						</label>
					</div>
					<div class="canvas-scroll">
						<SliceCanvas
							bind:this={canvas}
							doc={activeDoc}
							{slicePath}
							locale={displayLocale}
							{grid}
							{tool}
							{scope}
							{selection}
							onselect={onCanvasSelect}
							oncreate={(t, rect) => (t === 'text' ? addDialog(rect) : addSampleArea(rect))}
						/>
					</div>
				{:else}
					<p class="hint">{m['creator.select_slice']()}</p>
				{/if}
			</main>

			<!-- Inspector -->
			{#if section === 'slice' && activeDoc && activeSlice}
				<aside class="inspector">
					<div class="tabs" role="tablist">
						<button class="ghost" class:active={inspectorTab === 'text'} onclick={() => (inspectorTab = 'text')} data-testid="tab-text">{m['creator.text_boxes']()} ({sliceDialogs.length})</button>
						<button class="ghost" class:active={inspectorTab === 'items'} onclick={() => { inspectorTab = 'items'; loadCatalog(); }} data-testid="tab-items">{m['creator.slice_items']()} ({sliceItems.length})</button>
						<button class="ghost" class:active={inspectorTab === 'slice'} onclick={() => (inspectorTab = 'slice')} data-testid="tab-background">{m['creator.background']()}</button>
						<button class="ghost" class:active={inspectorTab === 'audio'} onclick={() => (inspectorTab = 'audio')}>{m['creator.audio']()}</button>
						<button class="ghost" class:active={inspectorTab === 'events'} onclick={() => (inspectorTab = 'events')}>Events ({activeSlice?.events?.length ?? 0})</button>
						<button class="ghost" class:active={inspectorTab === 'map'} onclick={() => (inspectorTab = 'map')} data-testid="tab-map">{m['creator.map']()}</button>
					</div>

					{#if inspectorTab === 'text'}
						<ul class="boxes">
							{#each sliceDialogs as d, di (di)}
								<li>
									<button class="ghost" class:active={selection?.kind === 'dialog' && selection.index === di} onclick={() => onCanvasSelect({ kind: 'dialog', index: di })} data-testid="dialog-{di}">
										<span>{d.id ?? `#${di + 1}`}</span> <span class="hint">{d.variant ?? 'bubble'}</span> {String(Array.isArray(d.text) ? d.text[0] : d.text).slice(0, 40)}
									</button>
								</li>
							{/each}
						</ul>
						<button class="secondary" onclick={() => addDialog()}><Icon name="plus" /> {m['creator.add_text_box']()}</button>
						{#if selection?.kind === 'dialog' && sliceDialogs[selection.index]}
							<hr />
							<DialogInspector
								doc={activeDoc}
								path={[...slicePath, 'dialog', selection.index]}
								editLocale={scope === 'locale' ? displayLocale : null}
								onremove={removeSelected}
							/>
						{/if}
					{:else if inspectorTab === 'items'}
						<ul class="boxes">
							{#each sliceItems as it, ii (ii)}
								<li>
									<button class="ghost" class:active={selection?.kind === 'item' && selection.index === ii} onclick={() => onCanvasSelect({ kind: 'item', index: ii })} data-testid="slice-item-{ii}">
										<span>{itemName(itemCatalog.catalog?.items.find((x) => x.id === it?.item), it?.item ?? '?')}</span> <span class="hint">{it?.action === 'pickup' ? m['creator.slice_item_action_pickup']() : it?.action === 'events' ? m['creator.slice_item_action_events']() : ''}</span>
									</button>
								</li>
							{:else}
								<li class="hint">{m['creator.slice_items_empty']()}</li>
							{/each}
						</ul>
						<select
							aria-label={m['creator.add_slice_item']()}
							title={m['creator.add_slice_item']()}
							value=""
							disabled={!itemCatalog.catalog}
							onchange={(e) => {
								addSliceItem(e.currentTarget.value);
								e.currentTarget.value = '';
							}}
							data-testid="add-slice-item"
						>
							<option value="">{m['creator.add_slice_item']()}</option>
							{#each itemCatalog.catalog?.items ?? [] as item (item.id)}<option value={item.id}>{itemName(item, item.id)} · {item.id}</option>{/each}
						</select>
						{#if selection?.kind === 'item' && sliceItems[selection.index]}
							<hr />
							<ItemInspector
								doc={activeDoc}
								path={[...slicePath, 'items', selection.index]}
								editLocale={scope === 'locale' ? displayLocale : null}
								{characters}
								{choices}
								background={activeSlice.image}
								onremove={removeSelected}
							/>
						{/if}
					{:else if inspectorTab === 'slice'}
						<SliceMedia doc={activeDoc} {slicePath} {toon} known={images} />
						<div class="row">
							<button class="secondary" onclick={() => moveSlice(-1)} disabled={sliceIndex === 0}>{m['creator.move_up']()}</button>
							<button class="secondary" onclick={() => moveSlice(1)} disabled={sliceIndex >= (activeGroup?.slices?.length ?? 0) - 1}>{m['creator.move_down']()}</button>
						</div>
					{:else if inspectorTab === 'audio'}
						<SliceAudio
							doc={activeDoc}
							{slicePath}
							chapterAudio={chapterData?.audio}
							selectedSample={selection?.kind === 'sample' ? selection.index : -1}
							onselectsample={(i) => (selection = i >= 0 ? { kind: 'sample', index: i } : null)}
						/>
					{:else if inspectorTab === 'events'}
						<VnEvents
							doc={activeDoc}
							eventsPath={[...slicePath, 'events']}
							sliceEvents
							{characters}
							{choices}
							background={activeSlice.image}
							selected={selection?.kind === 'event' ? selection.index : -1}
							onselect={(i) => (selection = { kind: 'event', index: i })}
						/>
					{:else if inspectorTab === 'map'}
						<SliceUnlocks mapDoc={worldMap} {toon} chapter={chapterNumber} image={activeSlice.image ?? ''} slices={chapterSlices(sceneGroups())} ontouch={() => (mapTouched = true)} />
					{/if}
				</aside>
			{/if}
		</div>
	{/if}

	{#if panel}
		<aside class="side-panel">
			{#if panel === 'problems'}
				<ProblemsPanel {problems} ongoto={gotoProblem} onclose={() => (panel = null)} />
			{:else if panel === 'history' && activeDoc}
				<HistoryPanel doc={section === 'group' || section === 'slice' ? activeDoc : chapter} onclose={() => (panel = null)} />
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
	.bar {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
		align-items: center;
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
		border-left: 0.5rem solid var(--ink);
	}
	.dirty-bar {
		background: var(--ink);
		color: var(--paper);
	}
	.warn-bar {
		border-left-color: var(--warn);
	}
	.error-bar {
		border-left-color: var(--error);
	}
	.note-bar {
		border-left-color: var(--cyan);
	}
	.columns {
		flex: 1;
		min-height: 0;
		display: grid;
		grid-template-columns: minmax(0, 1fr);
	}
	.columns.has-inspector {
		grid-template-columns: minmax(0, 1fr) 22rem;
	}
	/* narrower: the inspector goes below the canvas (sidebar 12rem + inspector 22rem + a 390px slice need ≈ 1280px) */
	@media (max-width: 1279px) {
		.columns.has-inspector {
			grid-template-columns: minmax(0, 1fr);
			grid-template-rows: minmax(0, 1fr) minmax(0, 50%);
		}
		.columns.has-inspector .inspector {
			border-left: 0;
			border-top: var(--border-width-regular) solid var(--ink);
		}
	}
	/* ── the chapter tree in the sidebar (entries: tools.css sc-nav-item / sc-nav-head) ── */
	.tree {
		padding: 0 0 var(--space-05);
	}
	.group-row {
		display: flex;
	}
	.group-row .node {
		flex: 1;
		min-width: 0;
		justify-content: space-between;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.node .hint {
		color: inherit;
	}
	.group-row .tools {
		display: none;
	}
	.group-row:hover .tools,
	.group.open .tools {
		display: flex;
	}
	.tools button {
		min-width: var(--control-height);
		padding: 0;
	}
	.group.open {
		border-top: var(--border-width-regular) solid var(--ink);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.tree-add {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		padding: var(--space-1);
	}
	/* one control height (owner 2026-10-02): the narrow sidebar keeps the labels on one line, the full text in the tooltip */
	.tree-add button {
		justify-content: flex-start;
		min-width: 0;
		padding-inline: var(--space-05);
	}
	.one-line {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.main {
		overflow: auto;
		padding: var(--space-1) var(--space-1) var(--space-3);
		min-width: 0;
	}
	.canvas-tools {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05) var(--space-1);
		align-items: center;
		position: sticky;
		top: calc(-1 * var(--space-1));
		background: var(--paper);
		z-index: 5;
		padding: var(--space-05) 0;
	}
	/* segmented control: one 2px ink frame, the chosen part inverted */
	.seg {
		display: inline-flex;
		border: var(--border-width-regular) solid var(--ink);
	}
	.inline {
		display: inline-flex !important;
		gap: var(--space-05);
		align-items: center;
		font-size: var(--type-caption-size);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.inline select {
		width: auto;
		font-weight: var(--font-weight-regular);
		text-transform: none;
		letter-spacing: normal;
	}
	.canvas-scroll {
		overflow: auto;
		padding: var(--space-1) 0 var(--space-15);
	}
	.inspector {
		overflow: auto;
		border-left: var(--border-width-regular) solid var(--ink);
		padding: var(--space-1) var(--space-1) var(--space-3);
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	/* inspector tabs: a framed grid, the open tab inverted */
	.tabs {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr));
		border: var(--border-width-regular) solid var(--ink);
	}
	.tabs button {
		justify-content: center;
	}
	.boxes {
		list-style: none;
		margin: 0;
		padding: 0;
		border: var(--border-width-regular) solid var(--ink);
	}
	.boxes li + li {
		border-top: var(--border-width-hairline) solid var(--ink);
	}
	/* one line per box, ellipsis (block, not flex: a text node can't ellipsize in flex) */
	.boxes button {
		display: block;
		width: 100%;
		padding-block: 0;
		line-height: calc(var(--control-height) - 2 * var(--border-width-regular));
		text-align: left;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.boxes button.active .hint {
		color: inherit;
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
	.sub {
		margin-top: var(--space-15);
	}
	hr {
		border: 0;
		border-top: var(--border-width-regular) solid var(--ink);
		width: 100%;
		margin: var(--space-05) 0;
	}
	.side-panel {
		position: fixed;
		top: var(--top);
		right: 0;
		/* ends at the bottom ActionBar (its border and last lines are not hidden behind it) */
		bottom: var(--action-bar-height);
		width: min(var(--tool-panel-width), 92vw);
		background: var(--paper);
		border-left: var(--border-width-regular) solid var(--ink);
		overflow: auto;
		padding: var(--space-1) var(--space-1) var(--space-3);
		z-index: 20;
		box-sizing: border-box;
	}
	.pad {
		padding: var(--space-1);
	}
	.error {
		color: var(--error);
	}
</style>
