<script lang="ts">
	import { confirmRemove } from '$lib/editor/ask.ts';
	import { CloseIcon } from '@scutoons/ui';
	import { setLookImage, type AssetRequests, type CharacterFileSlot, type CharacterPersona, type MoodVocabulary, type RegistryCharacter } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { addAssetRequest, edit, editCharacter, editPersona, ensureMoods } from '$lib/editor/characters.ts';
	import { cleanText, closeRequests, setLookApose } from '$lib/editor/character-form.ts';
	import { getCreatorSession } from '$lib/session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import LocaleMapField from '$lib/components/fields/locale-map-field.svelte';
	import ExpressionGrid from '$lib/components/characters/expression-grid.svelte';
	import AposeEditor from '$lib/components/characters/apose-editor.svelte';

	/**
	 * One look of a persona: its label, default / remove, the mood grid (ExpressionGrid) – setting a cell's image
	 * closes its open request, a request can be written per cell, a new mood joins the vocabulary – and the look's A-pose
	 * (the reader's avatar body in this look, AposeEditor).
	 */
	let {
		registry,
		moodsDoc,
		requestsDoc,
		moods,
		requests,
		character,
		persona,
		lookId,
		languages,
		source,
		bust,
		onlookchange
	}: {
		registry: ContentDoc;
		moodsDoc: ContentDoc;
		requestsDoc: ContentDoc;
		moods: MoodVocabulary;
		requests: AssetRequests | null;
		character: RegistryCharacter;
		persona: CharacterPersona;
		lookId: string;
		languages: string[];
		source: string;
		bust: number;
		/** the shown look goes away (removed): back to the persona's default */
		onlookchange: (look: string) => void;
	} = $props();

	const session = getCreatorSession();
	const id = $derived(character.id);
	const look = $derived(persona.looks.find((l) => l.id === lookId));
	const lookIds = $derived(persona.looks.map((l) => l.id));

	function change(fn: (p: CharacterPersona) => void, key?: string) {
		editPersona(registry, id, persona.id, fn, key ? `${id}:${persona.id}:${key}` : undefined);
	}
	function setDefault() {
		change((p) =>
			p.looks.forEach((l) => {
				if (l.id === lookId) l.default = true;
				else delete l.default;
			})
		);
	}
	async function remove() {
		if (lookIds.length < 2 || !(await confirmRemove(m['creator.characters_remove_look_confirm']({ look: lookId })))) return;
		const target = lookId;
		change((p) => {
			p.looks = p.looks.filter((l) => l.id !== target);
			if (!p.looks.some((l) => l.default)) p.looks[0].default = true;
			p.transitions = p.transitions?.filter((t) => t.from !== target && t.to !== target);
			if (!p.transitions?.length) delete p.transitions;
		});
		onlookchange('');
	}

	/** a cell's path; setting one also closes the cell's open request */
	function setImage(mood: string, view: string, path: string) {
		change((p) => {
			const l = p.looks.find((x) => x.id === lookId);
			if (l) setLookImage(l, mood, view, path);
		}, `img-${lookId}-${mood}-${view}`);
		if (path) closeRequests(requestsDoc, requests, id, [{ kind: 'expression', persona: persona.id, look: lookId, mood, view, path } as CharacterFileSlot]);
	}
	function request(mood: string, view: string, note: string) {
		const existing = requests?.requests.find((r) => r.status === 'open' && r.character === id && r.persona === persona.id && r.look === lookId && r.mood === mood && r.view === view);
		if (existing) edit<AssetRequests>(requestsDoc, (f) => f.requests.forEach((r) => r.id === existing.id && (note.trim() ? (r.note = note.trim()) : delete r.note)));
		else addAssetRequest(requestsDoc, { character: id, persona: persona.id, look: lookId, mood, view }, note, session.user?.email ?? null);
	}
	/** the look's A-pose picked (closes its open request) or cleared */
	function setApose(path: string | undefined) {
		const target = lookId;
		editCharacter(registry, id, (c) => setLookApose(c, persona.id, target, path, languages), `${id}:${persona.id}:apose-${target}`);
		if (path) closeRequests(requestsDoc, requests, id, [{ kind: 'apose', persona: persona.id, look: target, newLook: false, path }]);
	}
	function addMood(mood: string, label: string) {
		if (!moods.moods.some((x) => x.id === mood)) {
			ensureMoods(moodsDoc);
			edit<MoodVocabulary>(moodsDoc, (v) => v.moods.push({ id: mood, label: Object.fromEntries(languages.map((l, i) => [l, i === 0 ? label : ''])), order: Math.max(0, ...v.moods.map((x) => x.order ?? 0)) + 1 }));
		}
		change((p) => {
			const l = p.looks.find((x) => x.id === lookId);
			if (l) l.images[mood] ??= {};
		});
	}
</script>

{#if look}
	<div class="look-head">
		<LocaleMapField value={look.label} {languages} label={m['creator.characters_look_label']()} onchange={(v) => change((p) => { const l = p.looks.find((x) => x.id === lookId); if (!l) return; const t = cleanText(v); if (t) l.label = t; else delete l.label; }, `look-${lookId}`)} />
		<label class="check"><input type="radio" name="default-look-{id}-{persona.id}" checked={!!look.default} onchange={setDefault} /> {m['creator.characters_default_look']()}</label>
		{#if lookIds.length > 1}<button type="button" class="ghost" onclick={remove}><CloseIcon /> {m['creator.characters_remove_look']()}</button>{/if}
	</div>
	{#key `${persona.id}/${lookId}`}
		<ExpressionGrid {character} persona={persona.id} {look} {moods} {requests} locale={source} onset={setImage} onrequest={request} onaddmood={addMood} {bust} />
		<AposeEditor character={id} persona={persona.id} {look} {bust} onchange={setApose} />
	{/key}
{/if}

<style>
	.look-head {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		gap: 0.6rem;
		align-items: end;
	}
	.check {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: 0.3rem;
		font-weight: 400;
	}
</style>
