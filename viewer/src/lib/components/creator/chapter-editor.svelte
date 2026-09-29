<script lang="ts">
	import { onMount } from 'svelte';
	import { beforeNavigate } from '$app/navigation';
	import { API_BASE } from '$lib/config.ts';
	import { cookieName } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { ContentDoc, LOCALES, SaveConflict, SOURCE_LOCALE } from '$lib/creator/doc.svelte.ts';
	import { getAt, deleteAt, type Path } from '$lib/creator/paths.ts';
	import { newDialog, newGroup, nextDialogId } from '$lib/creator/defaults.ts';
	import { knownImages } from '$lib/creator/images.ts';
	import { validateFile, validateLocales, type Problem } from '$lib/creator/validate.ts';
	import { Trigger, TriggerMode } from '$lib/models/enums.ts';
	import SliceCanvas, { type CanvasSelection, type Scope, type Tool } from './slice-canvas.svelte';
	import SliceList from './slice-list.svelte';
	import DialogInspector from './dialog-inspector.svelte';
	import ImagePicker from './image-picker.svelte';
	import MetaPanel from './meta-panel.svelte';
	import GroupPanel from './group-panel.svelte';
	import AudioPanel from './audio-panel.svelte';
	import SliceAudio from './slice-audio.svelte';
	import VnEvents from './vn-events.svelte';
	import StyleGuide from './style-guide.svelte';
	import ProblemsPanel from './problems-panel.svelte';
	import HistoryPanel from './history-panel.svelte';
	import FileStatus from './file-status.svelte';

	let { toon, chapterNumber }: { toon: string; chapterNumber: number } = $props();

	// ── Documents ───────────────────────────────────────────────────────────────────────────────────────
	// the page re-creates this component per chapter ({#key}), so the initial props are the right ones
	// svelte-ignore state_referenced_locally
	const chapter = new ContentDoc(toon, `chapter${chapterNumber}.json`);
	let refs = $state<Record<string, ContentDoc>>({});
	let characters = $state<{ name: string; label?: string }[]>([]);
	let images = $state<string[]>([]);

	const chapterData = $derived(chapter.data[chapter.primary]);
	const groups = $derived<any[]>(chapterData?.groups ?? []);
	const allDocs = $derived<ContentDoc[]>([chapter, ...Object.values(refs)]);
	const anyDirty = $derived(allDocs.some((d) => d.dirty));
	const anySaving = $derived(allDocs.some((d) => d.saving));

	async function ensureRef(src: string) {
		if (refs[src]) return refs[src];
		const doc = new ContentDoc(toon, src);
		refs[src] = doc;
		await doc.load();
		return doc;
	}

	onMount(async () => {
		await chapter.load();
		await Promise.all(groups.filter((g) => g?.ref?.src).map((g) => ensureRef(g.ref.src)));
		try {
			const response = await fetch(`${API_BASE}/${toon}/${SOURCE_LOCALE}/characters.json`);
			if (response.ok) characters = await response.json();
		} catch {
			/* optional */
		}
		refreshImages();
	});

	function refreshImages() {
		knownImages(toon, LOCALES, allDocs.map((d) => $state.snapshot(d.data))).then((list) => (images = list));
	}

	// ── Selection / view state ──────────────────────────────────────────────────────────────────────────
	type Section = 'meta' | 'audio' | 'group' | 'slice';
	let section = $state<Section>('meta');
	let groupIndex = $state(0);
	let sliceIndex = $state(-1);
	let selection = $state<CanvasSelection | null>(null);
	let inspectorTab = $state<'text' | 'slice' | 'audio' | 'events'>('text');
	let displayLocale = $state<string>(SOURCE_LOCALE);
	let zoom = $state<1 | 3>(1);
	let grid = $state(0.5);
	let tool = $state<Tool>('text');
	let scope = $state<Scope>('all');
	let panel = $state<'guide' | 'problems' | 'history' | null>(null);
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
		else if (next?.kind === 'sample') inspectorTab = 'audio';
		else if (next?.kind === 'event') inspectorTab = 'events';
	}

	// ── Problems ────────────────────────────────────────────────────────────────────────────────────────
	const problems = $derived.by<Problem[]>(() => {
		const out: Problem[] = [];
		const names = characters.map((c) => c.name);
		for (const doc of allDocs) {
			if (doc.loading) continue;
			for (const l of doc.locales) {
				if (doc.source[l] === 'missing' && doc.data[l] == null) {
					out.push({ level: l === doc.primary ? 'error' : 'warning', file: doc.path, locale: l, path: [], message: `no ${l} file` });
					continue;
				}
				out.push(...validateFile(doc.kind, doc.data[l], l, { file: doc.path, audio: chapterData?.audio, characters: names.length ? names : undefined }));
			}
			out.push(...validateLocales(doc.kind, doc.path, doc.data, doc.primary));
		}
		const order = { error: 0, warning: 1, info: 2 };
		return out.sort((a, b) => order[a.level] - order[b.level]);
	});
	const errorCount = $derived(problems.filter((p) => p.level === 'error').length);
	const warningCount = $derived(problems.filter((p) => p.level === 'warning').length);

	function gotoProblem(p: Problem) {
		let path = p.path;
		if (p.file !== chapter.path) {
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
		requestAnimationFrame(() => document.querySelector<HTMLTextAreaElement>(`[data-testid="text-${SOURCE_LOCALE}"]`)?.focus());
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
			kind === 'dialog' ? [...slicePath, 'dialog'] : kind === 'sample' ? [...slicePath, 'audio', 'audio', 'samples'] : [...slicePath, 'events'];
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
			const src = prompt('Scene file (inside the chapter folder)', `chapter${chapterNumber}/${label}.json`)?.trim();
			if (!src) return;
			if (!/^[a-z0-9_-]+(\/[a-z0-9_-]+)?\.json$/.test(src)) {
				notice = 'The file name must look like chapter2/scene-21.json (a-z, 0-9, - and _).';
				return;
			}
			const doc = new ContentDoc(toon, src);
			doc.create(() => newGroup(src.split('/').pop()!.replace(/\.json$/, '')));
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
		for (const doc of allDocs) {
			if (!doc.dirty) continue;
			try {
				await doc.save(saveMessage.trim() || undefined, force);
			} catch (e) {
				if (e instanceof SaveConflict) conflict = { doc, error: e };
				else notice = `${doc.path}: ${e instanceof Error ? e.message : String(e)}`;
				return false;
			}
		}
		saveMessage = '';
		notice = m['creator.saved']();
		refreshImages();
		return true;
	}

	async function publishAll() {
		const docs = allDocs.filter((d) => d.dirty || d.unpublished.length || d.stored.length);
		const files = docs.map((d) => d.path).join(', ');
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
	const previewHref = $derived(
		`/${toon}/chapter/${chapterNumber}?preview=draft${previewLabel ? `&label=${encodeURIComponent(previewLabel)}` : ''}`
	);
	/** the reader's language is a cookie (paraglide strategy), not part of the URL: set it, then open the preview */
	function openPreview(event: MouseEvent, locale: string) {
		event.preventDefault();
		document.cookie = `${cookieName}=${locale}; path=/; max-age=34560000; SameSite=Lax`;
		window.open(previewHref, '_blank', 'noopener');
	}

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

<div class="editor" class:with-panel={panel !== null}>
	<header class="topbar">
		<nav class="crumbs">
			<a href="/creator">{m['creator.title']()}</a> / <a href="/creator/{toon}">{toon}</a> /
			<strong>{m['creator.chapters']()} {chapterNumber}{chapterData?.title ? ` · ${chapterData.title}` : ''}</strong>
		</nav>
		<div class="files">
			<FileStatus doc={chapter} />
			{#if refSrc && refs[refSrc]}<span class="hint">{refSrc}</span><FileStatus doc={refs[refSrc]} />{/if}
		</div>
		<div class="actions">
			<button class="ghost" title="{m['creator.undo']()} (Ctrl+Z)" disabled={!activeDoc?.canUndo} onclick={() => activeDoc?.undo()} data-testid="undo">↶</button>
			<button class="ghost" title="{m['creator.redo']()} (Ctrl+Shift+Z)" disabled={!activeDoc?.canRedo} onclick={() => activeDoc?.redo()} data-testid="redo">↷</button>
			<input class="message" type="text" placeholder="note (optional)" bind:value={saveMessage} />
			<button onclick={() => saveAll()} disabled={!anyDirty || anySaving} data-testid="save-draft">
				{anySaving ? m['creator.saving']() : m['creator.save_draft']()}{anyDirty ? ' •' : ''}
			</button>
			<button class="secondary" onclick={publishAll} disabled={anySaving} data-testid="publish">{m['creator.publish']()}</button>
			<span class="preview">
				{m['creator.preview']()}:
				{#each LOCALES as l}
					<a href={previewHref} target="_blank" rel="noopener" onclick={(e) => openPreview(e, l)} title="Draft preview in {l} (switches the reader language)" data-testid="preview-{l}">{l}</a>
				{/each}
			</span>
			<button class="ghost" class:active={panel === 'problems'} onclick={() => (panel = panel === 'problems' ? null : 'problems')} data-testid="problems-toggle">
				{m['creator.problems']()} <b class:bad={errorCount > 0}>{errorCount}</b>/<span>{warningCount}</span>
			</button>
			<button class="ghost" class:active={panel === 'history'} onclick={() => (panel = panel === 'history' ? null : 'history')}>{m['creator.history']()}</button>
			<button class="ghost" class:active={panel === 'guide'} onclick={() => (panel = panel === 'guide' ? null : 'guide')} data-testid="guide-toggle">{m['creator.style_guide']()}</button>
		</div>
	</header>

	{#if chapter.loading}
		<p class="pad">{m['creator.loading']()}</p>
	{:else if chapter.error}
		<p class="pad error">{chapter.error}</p>
	{:else}
		{#if anyDirty && !chapter.saving}<div class="bar dirty-bar">{m['creator.unsaved']()}{#if activeDoc?.dirty && section === 'slice'} · {activeDoc.dirtyLocales.join(', ')}{/if}</div>{/if}
		{#each recoverables as d (d.path)}
			<div class="bar warn-bar">
				{d.path}: {m['creator.recover_title']({ time: new Date(d.recoverable!.at).toLocaleString() })}
				{#if d.recoverableIsStale}<b>{m['creator.recover_stale']()}</b>{/if}
				<button class="secondary" onclick={() => d.recover()}>{m['creator.recover']()}</button>
				<button class="ghost" onclick={() => d.clearAutosave()}>{m['creator.discard']()}</button>
			</div>
		{/each}
		{#each missingLocales as { doc, locale } (doc.path + locale)}
			<div class="bar warn-bar">
				{doc.path}: {locale} {m['creator.missing']()}
				<button class="secondary" onclick={() => doc.createLocale(locale, (f) => (f && doc.kind === 'chapter' ? { ...f, language: locale } : f))}>
					{m['creator.create_locale']({ locale, source: doc.primary })}
				</button>
			</div>
		{/each}
		{#if conflict}
			<div class="bar error-bar">
				{m['creator.conflict']({ locale: conflict.error.locale, file: conflict.doc.path })}
				<button class="secondary" onclick={() => resolveConflict('reload')}>{m['creator.reload']()}</button>
				<button class="secondary" onclick={() => resolveConflict('overwrite')}>{m['creator.overwrite']()}</button>
			</div>
		{/if}
		{#if notice}<div class="bar note-bar">{notice} <button class="ghost" onclick={() => (notice = null)}>✕</button></div>{/if}

		<div class="columns" class:has-inspector={section === 'slice'}>
			<!-- Tree -->
			<nav class="tree" aria-label="chapter">
				<button class="node" class:current={section === 'meta'} onclick={() => (section = 'meta')} data-testid="tree-meta">{m['creator.meta']()}</button>
				<button class="node" class:current={section === 'audio'} onclick={() => (section = 'audio')} data-testid="tree-audio">{m['creator.audio']()}</button>
				<div class="node-head">{m['creator.groups']()}</div>
				{#each groups as g, gi (gi)}
					<div class="group" class:open={gi === groupIndex && (section === 'group' || section === 'slice')}>
						<div class="group-row">
							<button class="node" class:current={section === 'group' && gi === groupIndex} onclick={() => selectGroup(gi)} data-testid="group-{gi}">
								{#if g?.ref}↗ {g.ref.src}{:else}{g?.label ?? `group ${gi + 1}`}{/if}
								<span class="hint">{g?.ref ? '' : `${g?.slices?.length ?? 0}${g?.events ? ' · VN' : ''}`}</span>
							</button>
							<span class="tools">
								<button class="ghost" onclick={() => moveGroup(gi, -1)} disabled={gi === 0}>↑</button>
								<button class="ghost" onclick={() => moveGroup(gi, 1)} disabled={gi === groups.length - 1}>↓</button>
								<button class="ghost" onclick={() => removeGroup(gi)}>✕</button>
							</span>
						</div>
						{#if gi === groupIndex && (section === 'group' || section === 'slice')}
							{#if activeDoc && !activeDoc.loading}
								<SliceList doc={activeDoc} {groupPath} {chapterNumber} selected={section === 'slice' ? sliceIndex : -1} onselect={selectSlice} />
							{:else}
								<p class="hint pad">{m['creator.loading']()}</p>
							{/if}
						{/if}
					</div>
				{/each}
				<div class="tree-add">
					<button class="secondary" onclick={() => addGroup(false)}>+ {m['creator.add_group']()}</button>
					<button class="secondary" onclick={() => addGroup(true)}>+ {m['creator.add_ref_group']()}</button>
				</div>
			</nav>

			<!-- Main -->
			<main class="main">
				{#if section === 'meta'}
					<MetaPanel doc={chapter} {images} />
				{:else if section === 'audio'}
					<AudioPanel doc={chapter} />
				{:else if section === 'group'}
					{#if activeDoc && !activeDoc.loading}
						<GroupPanel doc={activeDoc} {groupPath} {refSrc} />
						{#if refSrc}
							<p class="hint">Scene file <code>{refSrc}</code> – edited here and saved/published as its own file (same versions per locale).</p>
						{/if}
						<h3 class="sub">{m['creator.vn']()} (group.events)</h3>
						{#if activeGroup?.events}
							<VnEvents doc={activeDoc} eventsPath={[...groupPath, 'events']} {characters} background={activeGroup?.slices?.[0]?.image ?? ''} />
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
						<span class="seg" role="group" aria-label="zoom">
							<button class="ghost" class:active={zoom === 1} onclick={() => (zoom = 1)} data-testid="zoom-1">1x</button>
							<button class="ghost" class:active={zoom === 3} onclick={() => (zoom = 3)} data-testid="zoom-3">3x</button>
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
							<button class="ghost" class:active={tool === 'text'} onclick={() => (tool = 'text')}>▭ text box</button>
							<button class="ghost" class:active={tool === 'sample'} onclick={() => (tool = 'sample')}>♪ sample area</button>
						</span>
						<label class="inline" title="Geometry edits on the canvas and in the fields">geometry
							<select bind:value={scope}>
								<option value="all">all locales</option>
								<option value="locale">only {displayLocale}</option>
							</select>
						</label>
						<span class="hint">{sliceIndex + 1} / {activeGroup?.slices?.length ?? 0}</span>
					</div>
					<p class="hint draw-hint">{m['creator.draw_hint']()}</p>
					<div class="canvas-scroll">
						<SliceCanvas
							bind:this={canvas}
							doc={activeDoc}
							{slicePath}
							locale={displayLocale}
							{zoom}
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
						<button class="ghost" class:active={inspectorTab === 'slice'} onclick={() => (inspectorTab = 'slice')}>{m['creator.image']()}</button>
						<button class="ghost" class:active={inspectorTab === 'audio'} onclick={() => (inspectorTab = 'audio')}>{m['creator.audio']()}</button>
						<button class="ghost" class:active={inspectorTab === 'events'} onclick={() => (inspectorTab = 'events')}>Events ({activeSlice?.events?.length ?? 0})</button>
					</div>

					{#if inspectorTab === 'text'}
						<ul class="boxes">
							{#each sliceDialogs as d, di (di)}
								<li>
									<button class="ghost" class:active={selection?.kind === 'dialog' && selection.index === di} onclick={() => onCanvasSelect({ kind: 'dialog', index: di })} data-testid="dialog-{di}">
										<b>{d.id ?? `#${di + 1}`}</b> <span class="hint">{d.variant ?? 'bubble'}</span> {String(Array.isArray(d.text) ? d.text[0] : d.text).slice(0, 40)}
									</button>
								</li>
							{/each}
						</ul>
						<button class="secondary" onclick={() => addDialog()}>+ {m['creator.add_text_box']()}</button>
						{#if selection?.kind === 'dialog' && sliceDialogs[selection.index]}
							<hr />
							<DialogInspector
								doc={activeDoc}
								path={[...slicePath, 'dialog', selection.index]}
								editLocale={scope === 'locale' ? displayLocale : null}
								onremove={removeSelected}
							/>
						{/if}
					{:else if inspectorTab === 'slice'}
						<ImagePicker toon={toon} value={activeSlice.image} known={images} showFormat onchange={(v) => activeDoc?.setShared([...slicePath, 'image'], v)} />
						{#each activeDoc.overrides([...slicePath, 'image']) as l}
							<p class="hint">{l}: {activeDoc.get(l, [...slicePath, 'image'])} ({m['creator.override']()})
								<button class="ghost" onclick={() => activeDoc?.resetOverride(l, [...slicePath, 'image'])}>{m['creator.use_shared']()}</button></p>
						{/each}
						<div class="row">
							<button class="secondary" onclick={() => moveSlice(-1)} disabled={sliceIndex === 0}>↑ move up</button>
							<button class="secondary" onclick={() => moveSlice(1)} disabled={sliceIndex >= (activeGroup?.slices?.length ?? 0) - 1}>↓ move down</button>
						</div>
					{:else if inspectorTab === 'audio'}
						<SliceAudio
							doc={activeDoc}
							{slicePath}
							chapterAudio={chapterData?.audio}
							selectedSample={selection?.kind === 'sample' ? selection.index : -1}
							onselectsample={(i) => (selection = i >= 0 ? { kind: 'sample', index: i } : null)}
						/>
					{:else}
						<VnEvents
							doc={activeDoc}
							eventsPath={[...slicePath, 'events']}
							sliceEvents
							{characters}
							background={activeSlice.image}
							selected={selection?.kind === 'event' ? selection.index : -1}
							onselect={(i) => (selection = { kind: 'event', index: i })}
						/>
					{/if}
				</aside>
			{/if}
		</div>
	{/if}

	{#if panel}
		<aside class="side-panel">
			{#if panel === 'guide'}
				<StyleGuide onclose={() => (panel = null)} />
			{:else if panel === 'problems'}
				<ProblemsPanel {problems} ongoto={gotoProblem} onclose={() => (panel = null)} />
			{:else if panel === 'history' && activeDoc}
				<HistoryPanel doc={section === 'group' || section === 'slice' ? activeDoc : chapter} onclose={() => (panel = null)} />
			{/if}
		</aside>
	{/if}
</div>

<style>
	.editor {
		--top: 64px;
		display: flex;
		flex-direction: column;
		height: 100dvh;
		overflow: hidden;
	}
	.topbar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.4rem 1rem;
		min-height: var(--top);
		padding: 0.4rem 0.75rem 0.4rem 96px;
		border-bottom: 2px solid var(--ink);
		box-sizing: border-box;
	}
	.crumbs a {
		color: var(--ink);
	}
	.files {
		display: flex;
		gap: 0.4rem;
		align-items: center;
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
		align-items: center;
		margin-left: auto;
	}
	.message {
		width: 11rem !important;
	}
	.preview {
		font-size: 11px;
		display: inline-flex;
		gap: 0.3rem;
	}
	.preview a {
		color: var(--ink);
	}
	.bad {
		color: var(--error);
	}
	.bar {
		padding: 0.3rem 0.75rem;
		font-size: 12px;
		display: flex;
		gap: 0.5rem;
		align-items: center;
		border-bottom: 1px solid var(--border-subtle);
	}
	.dirty-bar {
		background: var(--ink);
		color: var(--paper);
	}
	.warn-bar {
		background: #fff8e6;
	}
	.error-bar {
		background: #fff0ee;
		border-bottom-color: var(--error);
	}
	.note-bar {
		background: #eafaff;
	}
	.columns {
		flex: 1;
		min-height: 0;
		display: grid;
		grid-template-columns: 250px minmax(0, 1fr);
	}
	.columns.has-inspector {
		grid-template-columns: 250px minmax(0, 1fr) 400px;
	}
	@media (max-width: 1150px) {
		.columns.has-inspector {
			grid-template-columns: 220px minmax(0, 1fr);
			grid-template-rows: minmax(0, 1fr) minmax(0, 45%);
		}
		.columns.has-inspector .inspector {
			grid-column: 2;
			border-left: 0;
			border-top: 2px solid var(--ink);
		}
		.columns.has-inspector .tree {
			grid-row: 1 / span 2;
		}
	}
	.tree {
		overflow: auto;
		border-right: 2px solid var(--ink);
		/* the reader's fixed logo toggle (top left) reaches below the top bar */
		padding: 2rem 0 0.4rem;
	}
	.node {
		display: flex;
		justify-content: space-between;
		gap: 0.3rem;
		width: 100%;
		text-align: left;
		background: transparent;
		color: var(--ink);
		border: 0;
		border-left: 3px solid transparent;
		padding: 0.35rem 0.6rem;
	}
	.node:hover {
		background: var(--surface-hover);
	}
	.node.current {
		border-left-color: var(--ink);
		font-weight: 700;
		background: var(--surface-hover);
	}
	.node-head {
		padding: 0.6rem 0.6rem 0.2rem;
		font-size: 10px;
		text-transform: uppercase;
		color: var(--muted);
	}
	.group-row {
		display: flex;
	}
	.group-row .node {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.group-row .tools {
		display: none;
	}
	.group-row:hover .tools,
	.group.open .tools {
		display: flex;
	}
	.tools button {
		padding: 0 0.3em;
	}
	.group.open {
		background: #fafafa;
		border-top: 1px solid var(--border-subtle);
		border-bottom: 1px solid var(--border-subtle);
	}
	.tree-add {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		padding: 0.6rem;
	}
	.main {
		overflow: auto;
		padding: 0.75rem 1rem 3rem;
		min-width: 0;
	}
	.canvas-tools {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem 0.9rem;
		align-items: center;
		position: sticky;
		top: -0.75rem;
		background: var(--paper);
		z-index: 5;
		padding: 0.3rem 0;
	}
	.seg {
		display: inline-flex;
		border: 1px solid var(--border-muted);
	}
	.inline {
		display: inline-flex !important;
		gap: 0.3rem;
		align-items: center;
		font-size: 11px;
	}
	.inline select {
		width: auto;
	}
	.draw-hint {
		margin: 0.2rem 0 0.6rem;
	}
	.canvas-scroll {
		overflow: auto;
		padding: 1.2rem 0.5rem 2rem;
	}
	.inspector {
		overflow: auto;
		border-left: 2px solid var(--ink);
		padding: 0.5rem 0.75rem 3rem;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.tabs {
		display: flex;
		flex-wrap: wrap;
		border-bottom: 2px solid var(--ink);
		margin-bottom: 0.3rem;
	}
	.boxes {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.boxes button {
		width: 100%;
		text-align: left;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.row {
		display: flex;
		gap: 0.4rem;
	}
	.sub {
		font-size: 12px;
		margin-top: 1rem;
	}
	hr {
		border: 0;
		border-top: 2px solid var(--ink);
		width: 100%;
	}
	.side-panel {
		position: fixed;
		top: var(--top);
		right: 0;
		bottom: 0;
		width: min(24rem, 92vw);
		background: var(--paper);
		border-left: 2px solid var(--ink);
		overflow: auto;
		padding: 0.5rem 0.75rem 2rem;
		z-index: 20;
		box-sizing: border-box;
	}
	.pad {
		padding: 1rem;
	}
	.error {
		color: var(--error);
	}
</style>
