<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getAt, clone, type Path } from '@scutoons/content';
	import { JumpToTargetType, OnomatopoeiaEffect, OnomatopoeiaVariant, Trigger, TriggerMode } from '@scutoons/content';
	import PropField from './prop-field.svelte';
	import LocaleText from './locale-text.svelte';
	import OnomatopoeiaPreview from './onomatopoeia-preview.svelte';
	import CharacterRefField, { type CharacterChoices } from './characters/character-ref-field.svelte';
	import {
		REGULAR_PERSONA,
		resolveCharacterId,
		characterIndex,
		defaultLookId,
		defaultMoodVocabulary,
		findLook,
		findPersona,
		lookLabel,
		lookPortraitMoods,
		moodLabel,
		sortedMoods
	} from '@scutoons/content';

	/** the view of a joined character is its slot in the reader (DESIGN.md → "VN layout") */
	const VIEW_SLOTS = [
		{ value: 'avatar', label: 'Avatar (links)' },
		{ value: 'portrait', label: 'Portrait (rechts)' },
		{ value: 'full', label: 'Full (Mitte)' }
	];

	/**
	 * Visual-novel events: group scripts (`group.events`) or slice-triggered events (`slice.events`, with `trigger`).
	 * Structured forms for the common types; raw JSON (per locale) for everything else.
	 */
	let {
		doc,
		eventsPath,
		sliceEvents = false,
		characters = [],
		choices,
		background = '',
		selected = -1,
		onselect
	}: {
		doc: ContentDoc;
		eventsPath: Path;
		sliceEvents?: boolean;
		characters?: { name: string; label?: string }[];
		/** the toon's cast + registry: character fields become a cast select with "Add from registry…" / "New character…" */
		choices?: CharacterChoices;
		/** slice image for the onomatopoeia preview */
		background?: string;
		selected?: number;
		onselect?: (index: number) => void;
	} = $props();

	const FORM_TYPES = ['showMessage', 'addChoice', 'showChoices', 'hideMessage', 'jumpTo', 'showOnomatopeia', 'joinScene', 'leaveScene'];

	const events = $derived<any[]>(doc.get(doc.primary, eventsPath) ?? []);
	let raw = $state<Record<number, boolean>>({});
	let rawError = $state<Record<string, string>>({});

	function template(type: string): any {
		switch (type) {
			case 'showMessage':
				return { type, params: { text: '', wait: true } };
			case 'addChoice':
				return { type, params: { label: '' } };
			case 'jumpTo':
				return { type, params: { target: { type: 'chapter', value: '1' } } };
			case 'showOnomatopeia':
				return { type, params: { text: 'BAM', position: { x: 50, y: 50 }, size: 3, effect: 'shiver', mode: 'poly' } };
			case 'joinScene':
			case 'leaveScene':
				return { type, params: { characters: [{ character: characters[0]?.name ?? '', persona: 'regular', ...(type === 'joinScene' ? { variant: 'avatar' } : {}) }] } };
			default:
				return { type };
		}
	}

	function add(type: string) {
		const event = template(type);
		if (sliceEvents) event.trigger = { type: Trigger.CLICK, rect: { x: 0, y: 0, width: 100, height: 100 } };
		doc.structural((file) => {
			let list = getAt(file, eventsPath);
			if (!Array.isArray(list)) {
				const parent = getAt(file, eventsPath.slice(0, -1));
				if (!parent) return;
				list = parent[eventsPath[eventsPath.length - 1]] = [];
			}
			list.push(clone(event));
		});
		onselect?.(events.length - 1);
	}

	function remove(index: number) {
		doc.structural((file) => {
			const list = getAt(file, eventsPath);
			if (Array.isArray(list)) list.splice(index, 1);
		});
	}

	function moveBy(index: number, by: number) {
		const to = index + by;
		if (to < 0 || to >= events.length) return;
		doc.structural((file) => {
			const list = getAt(file, eventsPath);
			if (!Array.isArray(list)) return;
			const [item] = list.splice(index, 1);
			list.splice(to, 0, item);
		});
	}

	function changeType(index: number, type: string) {
		if (!confirm(`Change event ${index + 1} to ${type}? Its parameters are replaced.`)) return;
		doc.structural((file) => {
			const list = getAt(file, eventsPath);
			if (!Array.isArray(list)) return;
			const next = template(type);
			if (list[index]?.trigger) next.trigger = list[index].trigger;
			list[index] = next;
		});
	}

	function onSelectKind(event: any): string {
		const s = event?.params?.onSelect;
		if (!s) return 'none';
		if (s.set) return 'set';
		if (s.add) return 'add';
		if (s.jumpTo) return 'jumpTo';
		return 'other';
	}

	function setOnSelect(index: number, kind: string) {
		const path = [...eventsPath, index, 'params', 'onSelect'];
		const value =
			kind === 'set'
				? { set: { variable: '', value: '' } }
				: kind === 'add'
					? { add: { variable: '', value: 1 } }
					: kind === 'jumpTo'
						? { jumpTo: { target: { type: 'chapter', value: '1' } } }
						: undefined;
		doc.structural((file) => {
			const params = getAt(file, path.slice(0, -1));
			if (!params) return;
			if (value === undefined) delete params.onSelect;
			else params.onSelect = clone(value);
		});
	}

	function characterRows(index: number): any[] {
		return getAt(events[index], ['params', 'characters']) ?? [];
	}

	function addCharacter(index: number) {
		doc.structural((file) => {
			const params = getAt(file, [...eventsPath, index, 'params']);
			if (!params) return;
			params.characters ??= [];
			params.characters.push({ character: characters[0]?.name ?? '', persona: 'regular', variant: 'avatar' });
		});
	}

	function removeCharacter(index: number, ci: number) {
		doc.structural((file) => getAt(file, [...eventsPath, index, 'params', 'characters'])?.splice(ci, 1));
	}

	function rawValue(locale: string, index: number) {
		return JSON.stringify(doc.get(locale, [...eventsPath, index]) ?? null, null, 2);
	}

	function setRaw(locale: string, index: number, text: string) {
		const key = `${locale}:${index}`;
		try {
			const value = JSON.parse(text);
			doc.setLocal(locale, [...eventsPath, index], value);
			delete rawError[key];
		} catch (e) {
			rawError[key] = e instanceof Error ? e.message : String(e);
		}
	}

	const charOptions = $derived(characters.map((c) => ({ value: c.name, label: c.label ? `${c.name} (${c.label})` : c.name })));
	const vocabulary = $derived(choices?.moods ?? defaultMoodVocabulary());
	const choiceIndex = $derived(characterIndex(choices?.registry));
	/** the persona of a scene ref (registry) */
	const personaOf = (row: any) => {
		const r = resolveCharacterId(row?.character ?? row?.name ?? '', row?.persona);
		return findPersona(choiceIndex.get(r.character), r.persona || REGULAR_PERSONA);
	};
	/** the looks of the ref's persona (the default first) */
	function looksOf(row: any) {
		const p = personaOf(row);
		if (!p) return [];
		const main = defaultLookId(p);
		return [...p.looks].sort((a, b) => Number(b.id === main) - Number(a.id === main)).map((l) => ({ id: l.id, label: lookLabel(p, l.id, choices?.locale ?? 'en') }));
	}
	/** moods with an image for the ref's persona / look first, then the rest of the vocabulary (they fall back) */
	function moodOptions(row: any) {
		const p = personaOf(row);
		const look = findLook(p, row?.look || defaultLookId(p));
		const available = lookPortraitMoods(look);
		const all = sortedMoods(vocabulary).map((x) => x.id);
		const locale = choices?.locale ?? 'en';
		return [
			...all.filter((x) => available.includes(x)).map((x) => ({ value: x, label: moodLabel(x, locale, vocabulary) })),
			...available.filter((x) => !all.includes(x)).map((x) => ({ value: x, label: x })),
			...all.filter((x) => !available.includes(x)).map((x) => ({ value: x, label: `${moodLabel(x, locale, vocabulary)} (→ neutral)` }))
		];
	}
</script>

<section class="vn-events" data-testid="vn-events">
	{#each events as event, i (i)}
		{@const p = [...eventsPath, i]}
		{@const known = FORM_TYPES.includes(event?.type)}
		<article class="event" class:selected={i === selected}>
			<header>
				<button type="button" class="ghost idx" onclick={() => onselect?.(i)}>{i + 1}</button>
				<select value={event?.type} onchange={(e) => changeType(i, e.currentTarget.value)}>
					{#each FORM_TYPES as t}<option value={t}>{t}</option>{/each}
					{#if !known}<option value={event?.type}>{event?.type} (raw)</option>{/if}
				</select>
				<span class="tools">
					<button type="button" class="ghost" class:active={raw[i] || !known} title="raw JSON" onclick={() => (raw[i] = !raw[i])} disabled={!known}>{'{ }'}</button>
					<button type="button" class="ghost" onclick={() => moveBy(i, -1)} disabled={i === 0}>↑</button>
					<button type="button" class="ghost" onclick={() => moveBy(i, 1)} disabled={i === events.length - 1}>↓</button>
					<button type="button" class="ghost" onclick={() => remove(i)}>✕</button>
				</span>
			</header>

			{#if raw[i] || !known}
				<div class="raw">
					{#each doc.present as l (l)}
						<label>
							<span class="hint">{l}{l === doc.primary ? ' (source)' : ''}</span>
							<textarea rows="6" value={rawValue(l, i)} onchange={(e) => setRaw(l, i, e.currentTarget.value)} spellcheck="false"></textarea>
							{#if rawError[`${l}:${i}`]}<span class="err">{rawError[`${l}:${i}`]}</span>{/if}
						</label>
					{/each}
				</div>
			{:else if event.type === 'showMessage'}
				<div class="grid g3">
					{#if choices}
						<CharacterRefField {doc} path={[...p, 'params']} kind="speaker" label="character" {choices} placeholder="narrator" />
					{:else}
						<PropField {doc} path={[...p, 'params', 'character']} label="character" type="select" options={charOptions} placeholder="narrator" />
					{/if}
					<PropField {doc} path={[...p, 'params', 'wait']} label="wait (tap)" type="bool" placeholder="false" />
					<PropField {doc} path={[...p, 'params', 'mood']} label="mood" type="select" options={moodOptions(null)} placeholder="–" />
				</div>
				<LocaleText {doc} path={[...p, 'params', 'text']} rows={2} />
			{:else if event.type === 'addChoice'}
				<LocaleText {doc} path={[...p, 'params', 'label']} label="label" multiline={false} />
				<div class="grid g3">
					<div class="field">
						<label for="os-{i}">onSelect</label>
						<select id="os-{i}" value={onSelectKind(event)} onchange={(e) => setOnSelect(i, e.currentTarget.value)}>
							<option value="none">none</option>
							<option value="set">set variable</option>
							<option value="add">add to variable</option>
							<option value="jumpTo">jumpTo</option>
						</select>
					</div>
					{#if onSelectKind(event) === 'set'}
						<PropField {doc} path={[...p, 'params', 'onSelect', 'set', 'variable']} label="variable" required />
						<PropField {doc} path={[...p, 'params', 'onSelect', 'set', 'value']} label="value" required />
					{:else if onSelectKind(event) === 'add'}
						<PropField {doc} path={[...p, 'params', 'onSelect', 'add', 'variable']} label="variable" required />
						<PropField {doc} path={[...p, 'params', 'onSelect', 'add', 'value']} label="value" type="number" step={1} required />
					{:else if onSelectKind(event) === 'jumpTo'}
						<PropField {doc} path={[...p, 'params', 'onSelect', 'jumpTo', 'target', 'type']} label="target type" type="select" options={Object.values(JumpToTargetType)} required />
						<PropField {doc} path={[...p, 'params', 'onSelect', 'jumpTo', 'target', 'value']} label="value (chapter no.)" required />
						<PropField {doc} path={[...p, 'params', 'onSelect', 'jumpTo', 'target', 'label']} label="label (group)" />
					{/if}
				</div>
			{:else if event.type === 'jumpTo'}
				<div class="grid g3">
					<PropField {doc} path={[...p, 'params', 'target', 'type']} label="target type" type="select" options={Object.values(JumpToTargetType)} required />
					<PropField {doc} path={[...p, 'params', 'target', 'value']} label="value (chapter no.)" required />
					<PropField {doc} path={[...p, 'params', 'target', 'label']} label="label (group)" />
					<PropField {doc} path={[...p, 'params', 'wait']} label="wait (tap)" type="bool" placeholder="false" />
				</div>
			{:else if event.type === 'showOnomatopeia'}
				<LocaleText {doc} path={[...p, 'params', 'text']} rows={1} />
				<div class="ono">
					<div class="grid g3">
						<PropField {doc} path={[...p, 'params', 'position', 'x']} label="x %" type="number" placeholder="50" />
						<PropField {doc} path={[...p, 'params', 'position', 'y']} label="y %" type="number" placeholder="50" />
						<PropField {doc} path={[...p, 'params', 'size']} label="size (rem)" type="number" placeholder="2" />
						<PropField {doc} path={[...p, 'params', 'align']} label="align" type="select" options={['left', 'center', 'right']} placeholder="center" />
						<PropField {doc} path={[...p, 'params', 'rotation']} label="rotation °" type="number" step={1} placeholder="0" />
						<PropField {doc} path={[...p, 'params', 'variant']} label="variant" type="select" options={Object.values(OnomatopoeiaVariant)} placeholder="default" />
						<PropField {doc} path={[...p, 'params', 'effect']} label="effect" type="select" options={Object.values(OnomatopoeiaEffect)} placeholder="none" />
						<PropField {doc} path={[...p, 'params', 'duration']} label="duration ms" type="number" step={100} placeholder="2000" />
						<PropField {doc} path={[...p, 'params', 'delay']} label="delay ms" type="number" step={100} placeholder="0" />
						<PropField {doc} path={[...p, 'params', 'mode']} label="mode" type="select" options={Object.values(TriggerMode)} placeholder="mono" />
						<PropField {doc} path={[...p, 'params', 'wait']} label="wait (tap)" type="bool" placeholder="false" />
					</div>
					<OnomatopoeiaPreview params={event.params ?? {}} {background} toon={doc.toon} />
				</div>
			{:else if event.type === 'joinScene' || event.type === 'leaveScene'}
				{#each characterRows(i) as row, ci (ci)}
					{@const looks = looksOf(row)}
					<div class="grid g4">
						{#if choices}
							<CharacterRefField {doc} path={[...p, 'params', 'characters', ci]} label="character" {choices} />
						{:else}
							<PropField {doc} path={[...p, 'params', 'characters', ci, 'name']} label="character" type="select" options={charOptions} required />
						{/if}
						{#if event.type === 'joinScene'}
							<PropField {doc} path={[...p, 'params', 'characters', ci, row?.position && !row?.variant ? 'position' : 'variant', ...(row?.position && !row?.variant ? ['variant'] : [])]} label="Slot / Ansicht" type="select" options={VIEW_SLOTS} placeholder="Avatar (links)" />
							<PropField {doc} path={[...p, 'params', 'characters', ci, 'mood']} label="mood" type="select" options={moodOptions(row)} placeholder="neutral" />
							{#if looks.length > 1 || row?.look}
								<PropField {doc} path={[...p, 'params', 'characters', ci, 'look']} label="look" type="select" options={looks.slice(1).map((l) => ({ value: l.id, label: l.label }))} placeholder={looks[0] ? `${looks[0].label} (default)` : 'default'} />
							{/if}
						{/if}
						<button type="button" class="ghost" onclick={() => removeCharacter(i, ci)}>✕</button>
					</div>
				{/each}
				<button type="button" class="secondary" onclick={() => addCharacter(i)}>+ character</button>
			{:else}
				<p class="hint">No parameters.</p>
			{/if}

			{#if sliceEvents}
				<div class="grid g3 trigger">
					<PropField {doc} path={[...p, 'trigger', 'type']} label="trigger" type="select" options={Object.values(Trigger)} required />
					<PropField {doc} path={[...p, 'trigger', 'once']} label="once" type="bool" placeholder="false" />
					<span class="hint">{event.trigger?.rect ? 'area: drag it on the slice' : 'no area'}</span>
				</div>
			{/if}
		</article>
	{/each}

	<div class="add">
		{#each FORM_TYPES as t}
			<button type="button" class="secondary" onclick={() => add(t)}>+ {t}</button>
		{/each}
	</div>
</section>

<style>
	.vn-events {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.event {
		border: 1px solid var(--border-muted);
		padding: 0.4rem;
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}
	.event.selected {
		border: 2px solid var(--ink);
	}
	header {
		display: flex;
		gap: 0.4rem;
		align-items: center;
	}
	header select {
		width: auto;
	}
	.idx {
		font-weight: 700;
	}
	.tools {
		margin-left: auto;
		display: flex;
	}
	.grid {
		display: grid;
		gap: 0.3rem 0.4rem;
		align-items: end;
	}
	.g3 {
		grid-template-columns: repeat(3, minmax(0, 1fr));
	}
	.g4 {
		grid-template-columns: 2fr 1fr 1fr auto;
	}
	.ono {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		gap: 0.5rem;
	}
	.raw {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
		gap: 0.4rem;
	}
	.raw textarea {
		font-size: 11px;
	}
	.err {
		color: var(--error);
		font-size: 10px;
	}
	.field label {
		font-size: 10px;
		color: var(--muted);
	}
	.trigger {
		border-top: 1px dashed var(--border-muted);
		padding-top: 0.3rem;
	}
	.add {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
	}
	.add button {
		font-size: 11px;
		padding: 0.25em 0.6em;
	}
</style>
