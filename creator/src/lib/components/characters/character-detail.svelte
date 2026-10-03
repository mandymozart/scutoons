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
		personaIds,
		pickText,
		setLookImage,
		slugId,
		uniqueId,
		type AssetRequests,
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
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from '../credits/locale-map-field.svelte';
	import ExpressionGrid from './expression-grid.svelte';
	import AposeEditor from './apose-editor.svelte';
	import WantedImages from './wanted-images.svelte';

	/**
	 * One character: identity (name / description per language of its toons, color, own toon, voice, tags, artwork,
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
	let newArtwork = $state('');

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
	/** one A-pose look (character level); a path closes its open request */
	function setAposeLook(target: string, next: AposeLook, key?: string) {
		change((c) => {
			const looks = c.apose?.looks;
			const i = looks?.findIndex((l) => l.id === target) ?? -1;
			if (looks && i >= 0) looks[i] = next;
		}, key);
		if (next.path && requests?.requests.some((r) => r.status === 'open' && r.character === id && r.view === APOSE_VIEW && r.look === target)) {
			edit<AssetRequests>(requestsDoc, (f) =>
				f.requests.forEach((r) => {
					if (r.status === 'open' && r.character === id && r.view === APOSE_VIEW && r.look === target) {
						r.status = 'done';
						r.done_at = new Date().toISOString();
					}
				})
			);
		}
	}
	function requestApose(target: string, note: string) {
		const existing = requests?.requests.find((r) => r.status === 'open' && r.character === id && r.view === APOSE_VIEW && r.look === target);
		if (existing) edit<AssetRequests>(requestsDoc, (f) => f.requests.forEach((r) => r.id === existing.id && (note.trim() ? (r.note = note.trim()) : delete r.note)));
		else addAssetRequest(requestsDoc, { character: id, persona: REGULAR_PERSONA, look: target, mood: DEFAULT_MOOD, view: APOSE_VIEW }, note, session.user?.email ?? null);
	}
	/** the standard A-pose looks (swim default, naked – marked sensitive) for a character without any */
	function startApose() {
		change((c) => (c.apose = { looks: APOSE_LOOKS.map((l, i) => ({ id: l.id, label: { ...l.label }, ...(i === 0 ? { default: true } : {}), ...(l.sensitive ? { sensitive: true } : {}) })) }));
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

		<section class="block">
			<div class="sc-fields" data-cols="fill">
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
				<label class="wide">
					{m['creator.characters_image']()}
					<span class="inline">
						{#if character.image}<img class="mini-img" src={asset(character.image)} alt="" />{/if}
						<input type="text" value={character.image ?? ''} placeholder="catpope/characters/…" onchange={(e) => setField('image', e.currentTarget.value.trim())} />
					</span>
				</label>
			</div>
		</section>

		<!-- Personas → looks → moods -->
		<section class="block" data-testid="personas">
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
						<div class="sc-fields" data-cols="fit">
							<LocaleMapField value={currentPersona.label} {languages} label={m['creator.characters_persona_label']()} required testid="persona-label-text" onchange={(v) => changePersona((p) => (p.label = v), 'label')} />
							<div class="sc-fields intro" data-cols="fill">
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
							<ExpressionGrid {character} persona={personaId} look={currentLook} {moods} {requests} locale={source} onset={setImage} onrequest={request} onaddmood={addMood} />
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
									changePersona((p) => (p.transitions ??= []).push({ from, to, image: conventionTransitionPath(id, personaId, from, to) }));
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
			{#if character.apose?.looks.length}
				<AposeEditor
					character={id}
					looks={character.apose.looks}
					requests={(requests?.requests ?? []).filter((r) => r.character === id && r.view === APOSE_VIEW)}
					locale={source}
					onchange={setAposeLook}
					onrequest={requestApose}
				/>
			{:else}
				<p class="hint">{m['creator.characters_apose_none']()}</p>
				<button type="button" class="secondary" onclick={startApose} data-testid="start-apose"><Icon name="plus" /> {m['creator.characters_apose_start']()}</button>
			{/if}
		</section>

		<section class="block" data-testid="artwork">
			<h3>{m['creator.characters_artwork']()}</h3>
			<div class="artwork">
				{#each character.artwork ?? [] as a, i (a.id + i)}
					<figure data-testid="artwork-{a.id}">
						<img src={asset(a.path)} alt={pickText(a.label, source, languages) || a.id} loading="lazy" />
						<figcaption>
							<span>{pickText(a.label, source, languages) || a.id}</span>
							<input type="text" value={a.path} onchange={(e) => change((c) => c.artwork && (c.artwork[i].path = e.currentTarget.value.trim()))} />
							<button type="button" class="ghost" onclick={() => change((c) => { c.artwork?.splice(i, 1); if (!c.artwork?.length) delete c.artwork; })} aria-label={m['creator.remove']()} title={m['creator.remove']()}><CloseIcon /></button>
						</figcaption>
					</figure>
				{/each}
			</div>
			<span class="add">
				<input type="text" bind:value={newArtwork} placeholder="characters/{character.id}/{character.id}_keyvisual.png" />
				<button
					type="button"
					class="ghost"
					disabled={!newArtwork.trim()}
					onclick={() => {
						const path = newArtwork.trim();
						const aid = uniqueId(slugId(path.split('/').pop()?.replace(/\.[a-z]+$/i, '').replace(`${id}_`, '') ?? 'artwork'), (character.artwork ?? []).map((a) => a.id));
						change((c) => (c.artwork ??= []).push({ id: aid, path }));
						newArtwork = '';
					}}><Icon name="plus" /> {m['creator.characters_add_artwork']()}</button
				>
			</span>
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
	.wide {
		grid-column: 1 / -1;
	}
	.inline {
		display: flex;
		gap: 0.3rem;
		align-items: center;
		font-weight: 400;
	}
	.mini-img {
		width: 1.8rem;
		height: 1.8rem;
		object-fit: contain;
		flex: none;
		background: var(--surface-hover);
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
	/* the field grids' column widths (tools.css .sc-fields "fill" / "fit"): 1/8 rem under the defaults, so the wider page
	   gap (1rem) still fits as many columns as before (12rem / 18rem + 0.75rem) */
	.sc-fields[data-cols='fill'] {
		--fields-min: 11.875rem;
	}
	.sc-fields[data-cols='fit'] {
		--fields-min: 17.875rem;
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
	.artwork {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
	}
	figure {
		margin: 0;
		width: 12rem;
		border: 1px solid var(--border-subtle);
	}
	figure img {
		width: 100%;
		aspect-ratio: 1;
		object-fit: cover;
		display: block;
		background: var(--surface-hover);
	}
	figcaption {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		padding: 0.3rem;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
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
