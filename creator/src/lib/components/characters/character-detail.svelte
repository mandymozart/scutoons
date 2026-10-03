<script lang="ts">
	import { Badge, CloseIcon, Icon } from '@scutoons/ui';
	import {
		APOSE_LOOKS,
		APOSE_VIEW,
		DEFAULT_MOOD,
		REGULAR_PERSONA,
		castsOf,
		characterName,
		conventionTransitionPath,
		defaultLookId,
		findPersona,
		lookLabel,
		lookPortraitMoods,
		matchCharacterImageFile,
		newLook as lookEntry,
		personaIds,
		pickText,
		setLookImage,
		slugId,
		uniqueId,
		type AssetRequests,
		type CharacterFileSlot,
		type AposeLook,
		type CharacterRegistry,
		type CreditPerson,
		type LocaleText,
		type MoodVocabulary,
		type RegistryCharacter,
		type StoryPoint,
		type ToonCast
	} from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import {
		addAssetRequest,
		addLook,
		addPersona,
		characterLanguages,
		edit,
		editCharacter,
		editPersona,
		ensureMoods,
		registryOf,
		requestsOf,
		type ToonInfo
	} from '$lib/editor/characters.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { toWebp, uploadOrReplace } from '$lib/editor/media-upload.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from '../credits/locale-map-field.svelte';
	import ExpressionGrid from './expression-grid.svelte';
	import AposeEditor from './apose-editor.svelte';
	import WantedImages from './wanted-images.svelte';

	/**
	 * One character: identity (name / description per language of its toons, color, own toon, voice, tags,
	 * A-pose), then its personas as tabs → the persona's looks as sub-tabs → the mood grid
	 * of the look, plus the persona's transitions, and the character's wanted images.
	 */
	let {
		registry,
		moodsDoc,
		requestsDoc,
		moods,
		id,
		persona = $bindable(REGULAR_PERSONA),
		look = $bindable(''),
		toons,
		casts,
		people,
		onselect
	}: {
		registry: ContentDoc;
		moodsDoc: ContentDoc;
		requestsDoc: ContentDoc;
		moods: MoodVocabulary;
		id: string;
		persona?: string;
		look?: string;
		toons: ToonInfo[];
		casts: Record<string, ToonCast | null>;
		people: CreditPerson[];
		onselect: (id: string) => void;
	} = $props();

	const session = getCreatorSession();
	const file = $derived(registryOf(registry));
	const character = $derived(file?.characters.find((c) => c.id === id) ?? null);
	const appearsIn = $derived(character ? castsOf(character.id, casts) : []);
	const langs = $derived(characterLanguages(character, toons, appearsIn));
	const languages = $derived(langs.languages as string[]);
	const source = $derived(langs.source as string);
	const personas = $derived(personaIds(character));
	const personaId = $derived(personas.includes(persona) ? persona : REGULAR_PERSONA);
	const currentPersona = $derived(findPersona(character, personaId));
	const lookIds = $derived((currentPersona?.looks ?? []).map((l) => l.id));
	const lookId = $derived(lookIds.includes(look) ? look : defaultLookId(currentPersona));
	const currentLook = $derived(currentPersona?.looks.find((l) => l.id === lookId));
	const requests = $derived(requestsOf(requestsDoc));
	let newPersonaLabel = $state('');
	let newLookLabel = $state('');

	function change(fn: (c: RegistryCharacter) => void, key?: string) {
		editCharacter(registry, id, fn, key ? `${id}:${key}` : undefined);
	}
	function changePersona(fn: (p: NonNullable<typeof currentPersona>) => void, key?: string) {
		editPersona(registry, id, personaId, fn, key ? `${id}:${personaId}:${key}` : undefined);
	}
	/** set a field, removing it when empty */
	function setField<K extends keyof RegistryCharacter>(key: K, value: RegistryCharacter[K] | undefined | '') {
		change((c) => {
			if (value === undefined || value === '' || (Array.isArray(value) && !value.length)) delete c[key];
			else c[key] = value as RegistryCharacter[K];
		}, String(key));
	}
	const clean = (text: LocaleText) => (Object.values(text).some((t) => t?.trim()) ? text : undefined);
	function setPoint(p: StoryPoint | undefined, key: 'toon' | 'season' | 'chapter', raw: string): StoryPoint | undefined {
		const next: StoryPoint = { ...(p ?? { toon: '' }) };
		if (key === 'toon') next.toon = raw;
		else if (raw.trim() === '') delete next[key];
		else next[key] = Math.max(1, Math.round(Number(raw)) || 1);
		return next.toon ? next : undefined;
	}

	function remove() {
		if (!character) return;
		const used = appearsIn.length ? m['creator.characters_remove_used']({ toons: appearsIn.join(', ') }) : '';
		if (!confirm(`${m['creator.characters_remove_confirm']({ name: characterName(character, source) })} ${used}`.trim())) return;
		edit<CharacterRegistry>(registry, (f) => (f.characters = f.characters.filter((c) => c.id !== id)));
		onselect('');
	}
	function newPersona() {
		if (!newPersonaLabel.trim()) return;
		persona = addPersona(registry, id, newPersonaLabel, languages);
		look = '';
		newPersonaLabel = '';
	}
	function removePersona() {
		if (personaId === REGULAR_PERSONA || !confirm(m['creator.characters_remove_persona_confirm']({ persona: characterName(character, source, personaId) }))) return;
		change((c) => (c.personas = c.personas.filter((p) => p.id !== personaId)));
		persona = REGULAR_PERSONA;
	}
	function newLook() {
		if (!newLookLabel.trim()) return;
		look = addLook(registry, id, personaId, newLookLabel, languages);
		newLookLabel = '';
	}
	function setDefaultLook(target: string) {
		changePersona((p) =>
			p.looks.forEach((l) => {
				if (l.id === target) l.default = true;
				else delete l.default;
			})
		);
	}
	function removeLook(target: string) {
		if (lookIds.length < 2 || !confirm(m['creator.characters_remove_look_confirm']({ look: target }))) return;
		changePersona((p) => {
			p.looks = p.looks.filter((l) => l.id !== target);
			if (!p.looks.some((l) => l.default)) p.looks[0].default = true;
			p.transitions = p.transitions?.filter((t) => t.from !== target && t.to !== target);
			if (!p.transitions?.length) delete p.transitions;
		});
		look = '';
	}

	/** closes the open request of a cell (its image was set) */
	function closeRequest(mood: string, view: string) {
		if (!requests?.requests.some((r) => r.status === 'open' && r.character === id && r.persona === personaId && r.look === lookId && r.mood === mood && r.view === view)) return;
		edit<AssetRequests>(requestsDoc, (f) =>
			f.requests.forEach((r) => {
				if (r.status === 'open' && r.character === id && r.persona === personaId && r.look === lookId && r.mood === mood && r.view === view) {
					r.status = 'done';
					r.done_at = new Date().toISOString();
				}
			})
		);
	}
	/** the A-pose looks shown: the stored ones plus the standard looks not stored yet (empty slots) */
	const aposeLooks = $derived.by<AposeLook[]>(() => {
		const stored = character?.apose?.looks ?? [];
		const missing = APOSE_LOOKS.filter((l) => !stored.some((x) => x.id === l.id)).map((l) => ({ id: l.id, label: { ...l.label }, ...(l.sensitive ? { sensitive: true } : {}) }));
		return [...stored, ...missing];
	});
	/** an A-pose look's file (the look is created when it isn't stored; swim is the default) */
	function applyApose(c: RegistryCharacter, target: string, path: string) {
		c.apose ??= { looks: [] };
		let look = c.apose.looks.find((l) => l.id === target);
		if (!look) {
			const standard = APOSE_LOOKS.find((l) => l.id === target);
			look = { id: target, label: standard ? { ...standard.label } : { [languages[0] ?? 'en']: target }, ...(standard?.sensitive ? { sensitive: true } : {}) };
			c.apose.looks.push(look);
		}
		c.apose.looks.find((l) => l.id === target)!.path = path;
		if (!c.apose.looks.some((l) => l.default)) (c.apose.looks.find((l) => l.id === APOSE_LOOKS[0].id) ?? c.apose.looks[0]).default = true;
	}
	/** an A-pose slot picked (a path closes its open request) or cleared (an unused standard look disappears again) */
	function setAposePath(target: string, path: string | undefined) {
		change((c) => {
			if (path) return applyApose(c, target, path);
			const looks = c.apose?.looks;
			const look = looks?.find((l) => l.id === target);
			if (!looks || !look) return;
			delete look.path;
			if (!look.slots?.length && APOSE_LOOKS.some((l) => l.id === target)) looks.splice(looks.indexOf(look), 1);
			if (!looks.length) delete c.apose;
			else if (!looks.some((l) => l.default)) looks[0].default = true;
		}, `apose-${target}-path`);
		if (path) closeRequests([{ kind: 'apose', look: target, path }]);
	}
	/** closes the open requests the given files answer */
	function closeRequests(slots: CharacterFileSlot[]) {
		const answers = (r: AssetRequests['requests'][number]) =>
			r.status === 'open' &&
			r.character === id &&
			slots.some((s) =>
				s.kind === 'apose' ? r.view === APOSE_VIEW && r.look === s.look : s.kind === 'expression' && r.persona === s.persona && r.look === s.look && r.mood === s.mood && r.view === s.view
			);
		if (!requests?.requests.some(answers)) return;
		edit<AssetRequests>(requestsDoc, (f) =>
			f.requests.forEach((r) => {
				if (answers(r)) {
					r.status = 'done';
					r.done_at = new Date().toISOString();
				}
			})
		);
	}

	// ── import: every delivered file of the character onto its slot (owner 2026-10-03) ─────────────────────────────
	let importInput = $state<HTMLInputElement>();
	let importing = $state<{ done: number; total: number } | null>(null);
	let importSummary = $state('');
	let importProblems = $state<string[]>([]);
	/** set after an import: previews of files with new bytes at the same path load again */
	let bust = $state(0);

	function problemOf(slot: Extract<CharacterFileSlot, { kind: 'unknown' }>): string {
		if (slot.reason === 'other-character') return m['creator.characters_import_other']({ name: slot.name ?? '' });
		if (slot.reason === 'unknown-persona') return m['creator.characters_import_unknown_persona']({ name: slot.name ?? '' });
		if (slot.reason === 'unknown-look') return m['creator.characters_import_unknown_look']({ name: slot.name ?? '' });
		return m['creator.characters_import_no_slot']({ example: `${id}_portrait-neutral.png, ${id}_apose-swim.png` });
	}

	/**
	 * "Import images …": each file whose name says its slot (`matchCharacterImageFile`: expressions of any persona / look,
	 * A-pose looks, transitions) goes there as WebP at its own size (no density variants) under the convention path; a
	 * file already there gets the new bytes (a new version, the CDN purged). New looks and moods are added, answered
	 * requests closed – one undo step. Names for another character, a persona the character lacks or a transition
	 * between looks it lacks are reported, not guessed.
	 */
	async function importImages(event: Event) {
		const files = [...((event.currentTarget as HTMLInputElement).files ?? [])];
		if (importInput) importInput.value = '';
		if (!files.length || !character) return;
		importProblems = [];
		importSummary = '';
		const counts = { uploaded: 0, replaced: 0, unchanged: 0 };
		const done: Exclude<CharacterFileSlot, { kind: 'unknown' }>[] = [];
		importing = { done: 0, total: files.length };
		// transitions last, matched against the looks this import adds (`boy_helmet_portrait-…` + `boy_transition-helmet-to-…`)
		const known = { id: character.id, personas: character.personas.map((p) => ({ ...p, looks: [...p.looks] })) };
		const isTransition = (f: File) => /_transition-/i.test(f.name);
		for (const file of [...files.filter((f) => !isTransition(f)), ...files.filter(isTransition)]) {
			const slot = matchCharacterImageFile(file.name, known);
			if (slot.kind === 'expression' && slot.newLook) known.personas.find((p) => p.id === slot.persona)?.looks.push(lookEntry(slot.look, undefined, false));
			try {
				if (slot.kind === 'unknown') throw new Error(problemOf(slot));
				const bitmap = await createImageBitmap(file);
				let blob: Blob;
				try {
					blob = (await toWebp(bitmap)).blob;
				} finally {
					bitmap.close();
				}
				counts[await uploadOrReplace(blob, slot.path, 'character import')]++;
				done.push(slot);
			} catch (e) {
				importProblems = [...importProblems, `${file.name}: ${e instanceof Error ? e.message : String(e)}`];
			}
			importing = { done: importing.done + 1, total: files.length };
		}
		if (done.length) {
			const newMoods = [...new Set(done.flatMap((s) => (s.kind === 'expression' && !moods.moods.some((x) => x.id === s.mood) ? [s.mood] : [])))];
			if (newMoods.length) {
				ensureMoods(moodsDoc);
				edit<MoodVocabulary>(moodsDoc, (v) => {
					for (const mood of newMoods) v.moods.push({ id: mood, label: Object.fromEntries(languages.map((l, i) => [l, i === 0 ? mood : ''])), order: Math.max(0, ...v.moods.map((x) => x.order ?? 0)) + 1 });
				});
			}
			change((c) => {
				for (const s of done) {
					if (s.kind === 'apose') {
						applyApose(c, s.look, s.path);
						continue;
					}
					const p = c.personas.find((x) => x.id === s.persona);
					if (!p) continue;
					if (s.kind === 'expression') {
						// (push, then read back: the edit works on reactive proxies – a pushed object is not the stored one)
						if (!p.looks.some((x) => x.id === s.look)) p.looks.push(lookEntry(s.look, { [languages[0] ?? 'en']: s.look }, !p.looks.length));
						setLookImage(p.looks.find((x) => x.id === s.look)!, s.mood, s.view, s.path);
					} else {
						const t = p.transitions?.find((x) => x.from === s.from && x.to === s.to);
						if (t) t.image = s.path;
						else {
							p.transitions ??= [];
							p.transitions.push({ from: s.from, to: s.to, image: s.path });
						}
					}
				}
			});
			closeRequests(done);
		}
		importing = null;
		importSummary = m['creator.characters_import_summary'](counts);
		bust = Date.now();
	}

	/** a cell's path; setting one also closes the cell's open request */
	function setImage(mood: string, view: string, path: string) {
		changePersona((p) => {
			const l = p.looks.find((x) => x.id === lookId);
			if (l) setLookImage(l, mood, view, path);
		}, `img-${lookId}-${mood}-${view}`);
		if (path) closeRequest(mood, view);
	}
	function request(mood: string, view: string, note: string) {
		const existing = requests?.requests.find((r) => r.status === 'open' && r.character === id && r.persona === personaId && r.look === lookId && r.mood === mood && r.view === view);
		if (existing) edit<AssetRequests>(requestsDoc, (f) => f.requests.forEach((r) => r.id === existing.id && (note.trim() ? (r.note = note.trim()) : delete r.note)));
		else addAssetRequest(requestsDoc, { character: id, persona: personaId, look: lookId, mood, view }, note, session.user?.email ?? null);
	}
	function addMood(mood: string, label: string) {
		if (!moods.moods.some((x) => x.id === mood)) {
			ensureMoods(moodsDoc);
			edit<MoodVocabulary>(moodsDoc, (v) => v.moods.push({ id: mood, label: Object.fromEntries(languages.map((l, i) => [l, i === 0 ? label : ''])), order: Math.max(0, ...v.moods.map((x) => x.order ?? 0)) + 1 }));
		}
		changePersona((p) => {
			const l = p.looks.find((x) => x.id === lookId);
			if (l) l.images[mood] ??= {};
		});
	}

	/** any CSS colour (#hex or a name like 'lightblue') → #rrggbb for the native colour picker */
	function toHex(color: string | undefined): string {
		if (!color) return '#000000';
		if (/^#[0-9a-f]{6}$/i.test(color)) return color.toLowerCase();
		if (typeof document === 'undefined') return '#000000';
		const ctx = document.createElement('canvas').getContext('2d');
		if (!ctx) return '#000000';
		ctx.fillStyle = '#000000';
		ctx.fillStyle = color;
		return /^#[0-9a-f]{6}$/i.test(ctx.fillStyle) ? ctx.fillStyle : '#000000';
	}
</script>

{#if character}
	<article class="detail" data-testid="character-detail" data-id={character.id}>
		<header class="head">
			<div>
				<span class="type">{m['creator.characters_character']()}</span>
				<h2 data-testid="character-title">{characterName(character, source)}</h2>
				<span class="hint">id <code>{character.id}</code> · {m['creator.characters_languages']({ languages: languages.join(', '), source })}</span>
			</div>
			<button type="button" class="ghost danger" onclick={remove} data-testid="remove-character">{m['creator.characters_remove']()}</button>
		</header>
		<p class="relation" data-testid="appears-in">
			<span>{m['creator.characters_appears_in']()}:</span>
			{#each appearsIn as t (t)}<a href="/{t}/cast">{toons.find((x) => x.id === t)?.name ?? t}</a>{:else}<span class="hint">{m['creator.characters_in_no_cast']()}</span>{/each}
		</p>
		<!-- Creator roadmap (agents/AUTOMATION.md "Creator roadmap"): recorded, not built yet -->
		<p class="roadmap" data-testid="character-actions">
			<span>{m['creator.actions_title']()}:</span>
			{#each [m['creator.action_base_views'](), m['creator.action_moods'](), m['creator.action_upload']()] as label (label)}
				<button type="button" class="secondary" disabled title={m['creator.coming_soon']()}>{label} ({m['creator.coming_soon']()})</button>
			{/each}
		</p>

		<section class="block">
			<LocaleMapField value={character.name} {languages} label={m['creator.characters_name']()} required testid="character-name" onchange={(v) => change((c) => (c.name = v), 'name')} />
			<LocaleMapField value={character.description} {languages} label={m['creator.characters_description']()} multiline testid="character-description" onchange={(v) => setField('description', clean(v))} />
		</section>

		<section class="block grid g3">
			<label>
				{m['creator.characters_color']()}
				<span class="inline">
					<input type="color" class="picker" value={toHex(character.color)} oninput={(e) => setField('color', e.currentTarget.value)} aria-label="{m['creator.characters_color']()} picker" data-testid="character-color-picker" />
					<input type="text" value={character.color ?? ''} oninput={(e) => setField('color', e.currentTarget.value.trim())} data-testid="character-color" />
				</span>
			</label>
			<!-- no male/female: a testo level 0–100 (owner 2026-10-01, ITEMS.md "Substances"); empty = not set -->
			<label>
				{m['creator.characters_testo']()} {character.testo ?? '–'}
				<span class="inline">
					<input type="range" min="0" max="100" step="1" value={character.testo ?? 50} oninput={(e) => setField('testo', Number(e.currentTarget.value))} data-testid="character-testo" />
					<button type="button" class="ghost" disabled={character.testo === undefined} onclick={() => setField('testo', undefined)} title={m['creator.characters_testo_clear']()} aria-label={m['creator.characters_testo_clear']()}><CloseIcon /></button>
				</span>
			</label>
			<label>
				{m['creator.characters_own_toon']()}
				<select value={character.own_toon ?? ''} onchange={(e) => setField('own_toon', e.currentTarget.value)} data-testid="character-own-toon">
					<option value="">–</option>
					{#each toons as t (t.id)}<option value={t.id}>{t.name} · {t.id}</option>{/each}
				</select>
			</label>
			<label>
				{m['creator.characters_voiced_by']()}
				<select value={character.voiced_by ?? ''} onchange={(e) => setField('voiced_by', e.currentTarget.value)} data-testid="character-voiced-by">
					<option value="">–</option>
					{#each people.filter((p) => !p.merged_into).sort((a, b) => a.name.localeCompare(b.name)) as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
					{#if character.voiced_by && !people.some((p) => p.id === character.voiced_by)}<option value={character.voiced_by}>{character.voiced_by} (?)</option>{/if}
				</select>
			</label>
			<label>
				{m['creator.characters_tags']()}
				<input type="text" value={(character.tags ?? []).join(', ')} onchange={(e) => setField('tags', e.currentTarget.value.split(',').map((t) => t.trim()).filter(Boolean))} />
			</label>
			<label>
				{m['creator.characters_tendencies']()}
				<input type="text" value={(character.tendencies ?? []).join(', ')} placeholder="psychedelics" onchange={(e) => setField('tendencies', e.currentTarget.value.split(',').map((t) => t.trim().toLowerCase()).filter(Boolean))} data-testid="character-tendencies" />
			</label>
		</section>

		<!-- Personas → looks → moods -->
		<section class="block" data-testid="personas">
			<div class="import">
				<button type="button" class="secondary" disabled={!!importing} onclick={() => importInput?.click()} title={m['creator.characters_import_hint']({ id })} data-testid="character-import">
					{importing ? m['creator.import_slices_progress']({ done: importing.done, total: importing.total }) : m['creator.characters_import']()}
				</button>
				<input bind:this={importInput} type="file" accept="image/png,image/jpeg,image/webp" multiple hidden onchange={importImages} />
				{#if importSummary}<span class="sc-caption" role="status">{importSummary}</span>{/if}
			</div>
			{#each importProblems as problem}<p class="error" role="alert">{problem}</p>{/each}
			<nav class="tabs" aria-label={m['creator.characters_personas']()}>
				{#each personas as p (p)}
					<button type="button" class="tab" class:active={p === personaId} onclick={() => ((persona = p), (look = ''))} data-testid="persona-{p}">
						{p === REGULAR_PERSONA ? m['creator.characters_regular']() : pickText(findPersona(character, p)?.label, source, languages) || p}
					</button>
				{/each}
				<span class="add">
					<input type="text" bind:value={newPersonaLabel} placeholder={m['creator.characters_persona_label_placeholder']()} onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), newPersona())} data-testid="new-persona" />
					<button type="button" class="ghost" onclick={newPersona} disabled={!newPersonaLabel.trim()} data-testid="add-persona"><Icon name="plus" /> {m['creator.characters_add_persona']()}</button>
				</span>
			</nav>
			{#if currentPersona}
				<div class="persona" data-testid="persona-panel" data-persona={personaId}>
					{#if personaId !== REGULAR_PERSONA}
						<div class="grid g2">
							<LocaleMapField value={currentPersona.label} {languages} label={m['creator.characters_persona_label']()} required testid="persona-label-text" onchange={(v) => changePersona((p) => (p.label = v), 'label')} />
							<div class="grid g3 intro">
								<label>
									{m['creator.characters_introduced']()}
									<select value={currentPersona.introduced?.toon ?? ''} onchange={(e) => changePersona((p) => { const x = setPoint(p.introduced, 'toon', e.currentTarget.value); if (x) p.introduced = x; else delete p.introduced; })}>
										<option value="">{m['creator.characters_always']()}</option>
										{#each toons as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
									</select>
								</label>
								{#each ['season', 'chapter'] as const as key (key)}
									<label>
										{key === 'season' ? m['creator.characters_season']() : m['creator.characters_chapter']()}
										<input type="number" min="1" disabled={!currentPersona.introduced} value={currentPersona.introduced?.[key] ?? ''} onchange={(e) => changePersona((p) => { const x = setPoint(p.introduced, key, e.currentTarget.value); if (x) p.introduced = x; })} />
									</label>
								{/each}
							</div>
						</div>
						<label class="persona-color">
							{m['creator.characters_persona_color']()}
							<span class="inline">
								<input type="color" class="picker" value={toHex(currentPersona.color || character.color)} oninput={(e) => { const v = e.currentTarget.value; changePersona((p) => { p.color = v; }, 'color'); }} aria-label="{m['creator.characters_persona_color']()} picker" data-testid="persona-color-picker" />
								<input type="text" value={currentPersona.color ?? ''} placeholder={character.color} oninput={(e) => changePersona((p) => { const v = e.currentTarget.value.trim(); if (v) p.color = v; else delete p.color; }, 'color')} data-testid="persona-color" />
							</span>
						</label>
						<label class="persona-color">
							{m['creator.characters_persona_testo']()} {currentPersona.testo ?? character.testo ?? '–'}
							<span class="inline">
								<input type="range" min="0" max="100" step="1" value={currentPersona.testo ?? character.testo ?? 50} oninput={(e) => { const v = Number(e.currentTarget.value); changePersona((p) => { p.testo = v; }, 'testo'); }} data-testid="persona-testo" />
								<button type="button" class="ghost" disabled={currentPersona.testo === undefined} onclick={() => changePersona((p) => { delete p.testo; }, 'testo')} title={m['creator.characters_testo_clear']()} aria-label={m['creator.characters_testo_clear']()}><CloseIcon /></button>
							</span>
						</label>
						<LocaleMapField value={currentPersona.description} {languages} label={m['creator.characters_description']()} multiline onchange={(v) => changePersona((p) => { const t = clean(v); if (t) p.description = t; else delete p.description; }, 'description')} />
						<button type="button" class="ghost danger" onclick={removePersona}>{m['creator.characters_remove_persona']()}</button>
					{/if}

					<nav class="tabs looks" aria-label={m['creator.characters_looks']()}>
						{#each currentPersona.looks as l (l.id)}
							{@const n = lookPortraitMoods(l).length}
							<button type="button" class="tab" class:active={l.id === lookId} class:empty-look={n === 0} onclick={() => (look = l.id)} data-testid="look-{l.id}">
								{lookLabel(currentPersona, l.id, source)}{#if l.default && currentPersona.looks.length > 1}<span class="hint"> · {m['creator.characters_default']()}</span>{/if}
								{#if n}<span class="tally">{m['creator.characters_moods_count']({ count: n })}</span>{:else}<Badge tone="warning">{m['creator.characters_no_images']()}</Badge>{/if}
							</button>
						{/each}
						<span class="add">
							<input type="text" bind:value={newLookLabel} placeholder={m['creator.characters_new_look']()} onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), newLook())} data-testid="new-look" />
							<button type="button" class="ghost" onclick={newLook} disabled={!newLookLabel.trim()}><Icon name="plus" /> {m['creator.characters_add_look']()}</button>
						</span>
					</nav>
					{#if currentLook}
						<div class="look-head">
							<LocaleMapField value={currentLook.label} {languages} label={m['creator.characters_look_label']()} onchange={(v) => changePersona((p) => { const l = p.looks.find((x) => x.id === lookId); if (!l) return; const t = clean(v); if (t) l.label = t; else delete l.label; }, `look-${lookId}`)} />
							<label class="check"><input type="radio" name="default-look-{character.id}-{personaId}" checked={!!currentLook.default} onchange={() => setDefaultLook(lookId)} /> {m['creator.characters_default_look']()}</label>
							{#if lookIds.length > 1}<button type="button" class="ghost" onclick={() => removeLook(lookId)}><CloseIcon /> {m['creator.characters_remove_look']()}</button>{/if}
						</div>
						{#if !lookPortraitMoods(currentLook).length}
							<p class="empty-note" data-testid="look-empty">{m['creator.characters_look_empty_note']({ look: lookLabel(currentPersona, lookId, source) })}</p>
						{/if}
						{#key `${personaId}/${lookId}`}
							<ExpressionGrid {character} persona={personaId} look={currentLook} {moods} {requests} locale={source} onset={setImage} onrequest={request} onaddmood={addMood} {bust} />
						{/key}
					{/if}

					<div class="transitions" data-testid="transitions">
						<h3>{m['creator.characters_transitions']()}</h3>
						{#each currentPersona.transitions ?? [] as t, i (i)}
							<div class="transition">
								<img src={asset(t.image)} alt="" loading="lazy" />
								<div class="col">
									<span class="inline">
										<select value={t.from} onchange={(e) => changePersona((p) => p.transitions && (p.transitions[i].from = e.currentTarget.value))}>
											{#each lookIds as l (l)}<option value={l}>{lookLabel(currentPersona, l, source)}</option>{/each}
										</select>
										→
										<select value={t.to} onchange={(e) => changePersona((p) => p.transitions && (p.transitions[i].to = e.currentTarget.value))}>
											{#each lookIds as l (l)}<option value={l}>{lookLabel(currentPersona, l, source)}</option>{/each}
										</select>
									</span>
									<input type="text" value={t.image} onchange={(e) => changePersona((p) => p.transitions && (p.transitions[i].image = e.currentTarget.value.trim()))} />
									<LocaleMapField value={t.label} {languages} label={m['creator.characters_look_label']()} onchange={(v) => changePersona((p) => { if (!p.transitions) return; const x = clean(v); if (x) p.transitions[i].label = x; else delete p.transitions[i].label; }, `transition-${i}`)} />
								</div>
								<button type="button" class="ghost" onclick={() => changePersona((p) => { p.transitions?.splice(i, 1); if (!p.transitions?.length) delete p.transitions; })} aria-label={m['creator.remove']()} title={m['creator.remove']()}><CloseIcon /></button>
							</div>
						{:else}
							<p class="hint">{m['creator.characters_no_transitions_short']()}</p>
						{/each}
						{#if lookIds.length > 1}
							<button
								type="button"
								class="ghost"
								onclick={() => {
									const from = lookIds.find((l) => l !== defaultLookId(currentPersona)) ?? lookIds[0];
									const to = defaultLookId(currentPersona);
									changePersona((p) => {
										p.transitions ??= [];
										p.transitions.push({ from, to, image: conventionTransitionPath(id, personaId, from, to) });
									});
								}}
								data-testid="add-transition"><Icon name="plus" /> {m['creator.characters_add_transition']()}</button
							>
						{/if}
					</div>
				</div>
			{/if}
		</section>

		<section class="block" data-testid="apose">
			<h3>{m['creator.characters_apose']()}</h3>
			<AposeEditor character={id} looks={aposeLooks} locale={source} {bust} onchange={setAposePath} />
		</section>

		<section class="block">
			<h3>{m['creator.characters_wanted']()}</h3>
			<WantedImages registry={file} {moods} {requests} locale={source} character={id} onopen={(_, p, l) => ((persona = p), (look = l))} />
		</section>
	</article>
{:else}
	<p class="hint pad">{m['creator.characters_pick']()}</p>
{/if}

<style>
	.detail {
		display: flex;
		flex-direction: column;
		gap: 0.9rem;
	}
	.head {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 1rem;
	}
	.type {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--muted);
	}
	h2 {
		margin: 0.1rem 0 0.2rem;
	}
	h3 {
		margin: 0 0 0.4rem;
	}
	.relation {
		display: flex;
		flex-wrap: wrap;
		gap: 0.4rem;
		align-items: center;
		margin: 0;
	}
	.block {
		border-top: 1px solid var(--border-subtle);
		padding-top: 0.6rem;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}
	.grid {
		display: grid;
		gap: 0.5rem 0.75rem;
	}
	.g3 {
		grid-template-columns: repeat(auto-fill, minmax(12rem, 1fr));
	}
	.g2 {
		grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
	}
	.block.grid {
		display: grid;
	}
	label {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
		font-weight: var(--font-weight-regular);
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}
	.check {
		flex-direction: row;
		align-items: center;
		gap: 0.3rem;
		font-weight: 400;
	}
	.inline {
		display: flex;
		gap: 0.3rem;
		align-items: center;
		font-weight: 400;
	}
	.tabs {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
		align-items: center;
		border-bottom: 2px solid var(--ink);
		padding-bottom: 0.25rem;
	}
	.tabs.looks {
		border-bottom-width: 1px;
		margin-top: 0.5rem;
	}
	.tab {
		background: var(--paper);
		color: var(--ink);
		border: 2px solid var(--border-muted);
		padding: 0 0.6rem;
	}
	.tab.active {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	.tab.empty-look {
		border-style: dashed;
		border-color: var(--error);
	}
	.tally {
		margin-left: 0.35rem;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.add {
		display: inline-flex;
		gap: 0.25rem;
		align-items: center;
		margin-left: auto;
	}
	.add input {
		width: 12rem;
	}
	.persona {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}
	.intro {
		align-self: end;
	}
	.persona-color {
		max-width: 16rem;
	}
	.look-head {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		gap: 0.6rem;
		align-items: end;
	}
	.empty-note {
		margin: 0;
		padding: 0.3rem 0.5rem;
		border: 1px dashed var(--error);
		color: var(--error);
	}
	.transitions {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		align-items: flex-start;
	}
	.transition {
		display: grid;
		grid-template-columns: 6rem minmax(0, 1fr) auto;
		gap: 0.5rem;
		width: 100%;
	}
	.transition img {
		width: 6rem;
		height: 6rem;
		object-fit: cover;
		background: var(--surface-hover);
	}
	.col {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.import {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		align-items: center;
		margin-bottom: 1rem;
	}
	.danger {
		color: var(--error);
		align-self: flex-start;
	}
	.pad {
		padding: 1rem;
	}
	.roadmap {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.4rem;
	}
	/* the native colour picker as the swatch (square, the design's 2px rule) */
	.picker {
		width: 2rem;
		height: 2rem;
		padding: 0;
		border: 2px solid var(--border-muted, #ccc);
		background: transparent;
		cursor: pointer;
		flex: none;
	}
</style>
