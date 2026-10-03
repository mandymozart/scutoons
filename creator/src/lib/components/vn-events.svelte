<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { CloseIcon, Icon } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { getAt, clone, type Path } from '@scutoons/content';
	import { JumpToTargetType, OnomatopoeiaEffect, Trigger, TriggerMode } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { ONO_PRESETS, applyOnoPreset, matchOnoPreset, type OnoPreset } from '$lib/editor/event-presets.ts';
	import PropField from './prop-field.svelte';
	import { clickSelect, moveMany } from '$lib/editor/multi-select.ts';
	import { characterImage } from '@scutoons/content';
	import { isKnownMissing } from '$lib/editor/characters.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import LocaleText from './locale-text.svelte';
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
	 * Structured forms for every type the reader plays; no raw JSON (owner 2026-10-03) – another type is only named, to remove.
	 */
	let {
		doc,
		eventsPath,
		sliceEvents = false,
		characters = [],
		choices,
		selected = -1,
		onselect,
		dialogs = [],
		oncreatebox
	}: {
		doc: ContentDoc;
		eventsPath: Path;
		sliceEvents?: boolean;
		characters?: { name: string; label?: string }[];
		/** the toon's cast + registry: character fields become a cast select with "Add from registry…" / "New character…" */
		choices?: CharacterChoices;
		selected?: number;
		onselect?: (index: number) => void;
		/**
		 * text boxes an onomatopoeia event may reveal (`params.dialog`): the slice's boxes (slice events) or the boxes of
		 * the group's slices (a group script)
		 */
		dialogs?: { id: string; label: string }[];
		/** "Create a text box for this": a new text box on the slice, revealed by event `index` */
		oncreatebox?: (index: number) => void;
	} = $props();

	const FORM_TYPES = ['showMessage', 'addChoice', 'showChoices', 'hideMessage', 'jumpTo', 'showOnomatopeia', 'joinScene', 'leaveScene'];

	const events = $derived<any[]>(doc.get(doc.primary, eventsPath) ?? []);

	function template(type: string): any {
		switch (type) {
			case 'showMessage':
				return { type, params: { text: '', wait: true } };
			case 'addChoice':
				return { type, params: { label: '' } };
			case 'jumpTo':
				return { type, params: { target: { type: 'chapter', value: '1' } } };
			case 'showOnomatopeia':
				// the text box is picked (or created) in the form
				return { type, params: { effect: 'shiver' } };
			case 'joinScene':
			case 'leaveScene':
				return { type, params: { characters: [{ character: characters[0]?.name ?? '', persona: 'regular', ...(type === 'joinScene' ? { variant: 'avatar' } : {}) }] } };
			default:
				return { type };
		}
	}

	/** an onomatopoeia's preset (effect + timing), the text and mode / wait stay */
	function setOnoPreset(index: number, preset: OnoPreset) {
		doc.structural((file) => {
			const event = getAt(file, [...eventsPath, index]);
			if (!event) return;
			event.params ??= {};
			applyOnoPreset(event.params, preset);
		}, `${eventsPath.join('.')}.${index}.ono-preset`);
	}
	/** the trigger in words (owner 2026-10-03) */
	const TRIGGER_OPTIONS = $derived([
		{ value: Trigger.CLICK, label: m['creator.trigger_click']() },
		{ value: Trigger.HOVER, label: m['creator.trigger_hover']() },
		{ value: Trigger.INTERSECT, label: m['creator.trigger_intersect']() }
	]);

	/** whole slice = no `trigger.rect`; off = a centred area to drag on the slice */
	function setWholeSlice(index: number, whole: boolean) {
		doc.structural((file) => {
			const trigger = getAt(file, [...eventsPath, index, 'trigger']);
			if (!trigger) return;
			if (whole) delete trigger.rect;
			else trigger.rect = { x: 10, y: 30, width: 80, height: 40 };
		}, `${eventsPath.join('.')}.${index}.area`);
		onselect?.(index);
	}

	function add(type: string) {
		const event = template(type);
		// a new slice event covers the whole slice (when scrolled into view); an area is drawn with the ⚡ tool or switched on below
		if (sliceEvents) event.trigger = { type: Trigger.INTERSECT };
		doc.structural((file) => {
			let list = getAt(file, eventsPath);
			if (!Array.isArray(list)) {
				const parent = getAt(file, eventsPath.slice(0, -1));
				if (!parent) return;
				list = parent[eventsPath[eventsPath.length - 1]] = [];
			}
			list.push(clone(event));
		});
		const index = events.length - 1;
		open = [...open, index];
		picked = [index];
		anchor = index;
		onselect?.(index);
		void focusMain(index, type);
	}

	/**
	 * A new event opens with the cursor where the work is (owner 2026-10-03): the message text, the choice label, the
	 * character to join / leave (its picker), else the first field after the type.
	 */
	async function focusMain(index: number, type: string) {
		await tick();
		const root = document.querySelector<HTMLElement>(`[data-testid="event-${index}"]`);
		if (!root) return;
		const first = (selector: string) => root.querySelector<HTMLElement>(selector);
		const target =
			(type === 'showMessage' && first('textarea')) ||
			((type === 'joinScene' || type === 'leaveScene') && first('[data-testid="character-ref-open"]')) ||
			[...root.querySelectorAll<HTMLElement>('input:not([type="checkbox"]), textarea, select, button[data-testid]')].find(
				(el) => !el.closest('.type-field') && !el.closest('header')
			);
		target?.focus();
		root.scrollIntoView({ block: 'nearest' });
	}

	function remove(index: number) {
		doc.structural((file) => {
			const list = getAt(file, eventsPath);
			if (Array.isArray(list)) list.splice(index, 1);
		});
	}

	// ── one-liners (owner 2026-10-03): an event is a line (number, type, what it does) until opened; the selected one opens ──
	let open = $state<number[]>([]);
	$effect(() => {
		if (selected >= 0 && !untrack(() => open).includes(selected)) open = [...untrack(() => open), selected];
	});
	const isOpen = (i: number) => open.includes(i);

	// ── multi-select + drag (like the chapter tree): click = one (opens / folds it), Shift = range, Ctrl / Cmd = add / remove ──
	let picked = $state<number[]>([]);
	let anchor = $state<number | null>(null);
	let dragging = $state<number[] | null>(null);
	let dragOver = $state<number | null>(null);
	function clickRow(index: number, event: MouseEvent) {
		const next = clickSelect(picked, anchor, index, event);
		picked = next.selected;
		anchor = next.anchor;
		if (event.shiftKey || event.ctrlKey || event.metaKey) return;
		open = isOpen(index) ? open.filter((i) => i !== index) : [...open, index];
		onselect?.(index);
	}
	function dropOn(target: number) {
		const moving = dragging;
		if (!moving?.length) return;
		let map = new Map<number, number>();
		doc.structural((file) => {
			const list = getAt(file, eventsPath);
			if (!Array.isArray(list)) return;
			const result = moveMany(list, moving, target);
			map = result.map;
			list.splice(0, list.length, ...result.list);
		});
		const to = (i: number) => map.get(i) ?? i;
		picked = moving.map(to).sort((a, b) => a - b);
		open = open.map(to);
		anchor = anchor === null ? null : to(anchor);
		if (selected >= 0) onselect?.(to(selected));
	}

	/** the line's words: what the event does, from the source language */
	function summary(event: any): string {
		const p = event?.params ?? {};
		const cut = (s: unknown, n = 48) => {
			const t = String(Array.isArray(s) ? s[0] : (s ?? '')).replace(/\s+/g, ' ').trim();
			return t.length > n ? `${t.slice(0, n)}…` : t;
		};
		switch (event?.type) {
			case 'showMessage':
				// the speaker is the small avatar in front of the line (speakerAvatar); the words in quotes
				return p.text ? `“${cut(p.text)}”` : '';
			case 'addChoice':
				return `“${cut(p.label)}”`;
			case 'jumpTo':
				return `→ ${p.target?.type ?? ''} ${p.target?.value ?? ''}${p.target?.label ? ` · ${p.target.label}` : ''}`;
			case 'showOnomatopeia': {
				const preset = matchOnoPreset(p);
				return [p.dialog, preset ? m[`creator.ono_preset_${preset}`]() : ''].filter(Boolean).join(' · ');
			}
			case 'joinScene':
			case 'leaveScene':
				// the avatars say who (people())
				return '';
			default:
				return '';
		}
	}
	/** the speaker's avatar (a tiny one in the line, owner 2026-10-03); none without a registry image */
	const registryIndex = $derived(choices ? characterIndex(choices.registry) : null);
	function avatarOf(raw: any): string {
		const id = raw?.character ?? raw?.name;
		if (!registryIndex || !id) return '';
		const ref = resolveCharacterId(id, raw.persona);
		const character = registryIndex.get(ref.character);
		const src = character ? characterImage(character, { persona: ref.persona, look: raw.look, mood: raw.mood }, 'avatar', { isMissing: isKnownMissing }) : '';
		return src && !src.includes('missing') ? asset(src) : '';
	}
	/** who the line is about: the speaker (showMessage) or everyone joining / leaving – avatar, else the name (owner 2026-10-03) */
	function people(event: any): { id: string; src: string }[] {
		const refs = event?.type === 'showMessage' ? (event.params?.character ? [event.params] : []) : event?.type === 'joinScene' || event?.type === 'leaveScene' ? (event.params?.characters ?? []) : [];
		return refs.map((r: any) => ({ id: String(r?.character ?? r?.name ?? ''), src: avatarOf(r) })).filter((p: { id: string }) => p.id);
	}
	const triggerWord = (event: any) => TRIGGER_OPTIONS.find((o) => o.value === event?.trigger?.type)?.label ?? '';

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

	/** a new row, empty, its picker focused (owner 2026-10-03: the button "did not work" – it copied the first character) */
	async function addCharacter(index: number) {
		let row = 0;
		doc.structural((file) => {
			const params = getAt(file, [...eventsPath, index, 'params']);
			if (!params) return;
			params.characters ??= [];
			params.characters.push({ character: '', persona: 'regular', ...(getAt(file, [...eventsPath, index, 'type']) === 'joinScene' ? { variant: 'avatar' } : {}) });
			row = params.characters.length - 1;
		});
		await tick();
		document.querySelectorAll<HTMLElement>(`[data-testid="event-${index}"] [data-testid="character-ref-open"]`)[row]?.focus();
	}

	function removeCharacter(index: number, ci: number) {
		doc.structural((file) => getAt(file, [...eventsPath, index, 'params', 'characters'])?.splice(ci, 1));
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
			...all.filter((x) => !available.includes(x)).map((x) => ({ value: x, label: `${moodLabel(x, locale, vocabulary)} (neutral fallback)` }))
		];
	}
</script>

<section class="vn-events" data-testid="vn-events">
	{#each events as event, i (i)}
		{@const p = [...eventsPath, i]}
		{@const known = FORM_TYPES.includes(event?.type)}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<article
			class="event"
			class:selected={i === selected}
			class:open={isOpen(i)}
			class:picked={picked.length > 1 && picked.includes(i)}
			class:over={dragOver === i && !dragging?.includes(i)}
			class:below={dragOver === i && !!dragging?.length && i > Math.min(...dragging)}
			class:moving={dragging?.includes(i)}
			ondragover={(e) => {
				if (!dragging) return;
				e.preventDefault();
				dragOver = i;
			}}
			ondragleave={() => (dragOver = dragOver === i ? null : dragOver)}
			ondrop={(e) => {
				if (!dragging) return;
				e.preventDefault();
				dropOn(i);
				dragging = dragOver = null;
			}}
			data-testid="event-{i}"
		>
			<header
				draggable="true"
				ondragstart={(e) => {
					dragging = picked.includes(i) ? [...picked] : [i];
					if (!picked.includes(i)) picked = [i];
					e.dataTransfer?.setData('text/plain', dragging.join(','));
					if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
				}}
				ondragend={() => (dragging = dragOver = null)}
			>
				<button type="button" class="ghost line" onclick={(e) => clickRow(i, e)} aria-expanded={isOpen(i)} data-testid="event-line-{i}">
					<Icon name="chevron-right" />
					<span class="idx">{i + 1}</span>
					<span class="type">{event?.type}</span>
					{#each people(event) as who, wi (wi)}
						{#if who.src}<img class="avatar" src={who.src} alt={who.id} title={who.id} />{:else}<span class="who">{who.id}</span>{/if}
					{/each}
					<span class="sum">{summary(event)}</span>
					{#if sliceEvents && event?.trigger}<span class="trig">{triggerWord(event)}{event.trigger.rect ? '' : ` · ${m['creator.event_whole_slice']()}`}</span>{/if}
				</button>
				<span class="tools">
					<button type="button" class="ghost" onclick={() => remove(i)} aria-label={m['creator.remove']()} title={m['creator.remove']()}><CloseIcon /></button>
				</span>
			</header>

			{#if isOpen(i)}
			<label class="type-field">
				<span class="sc-label">type</span>
				<select value={event?.type} onchange={(e) => changeType(i, e.currentTarget.value)}>
					{#each FORM_TYPES as t}<option value={t}>{t}</option>{/each}
					{#if !known}<option value={event?.type}>{event?.type}</option>{/if}
				</select>
			</label>
			{#if !known}
				<p class="hint">{m['creator.event_unknown']({ type: String(event?.type) })}</p>
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
				{@const box = event.params?.dialog}
				<!-- an onomatopoeia is a text box this event reveals: the box gives the look, the event the timing -->
				<div class="grid g3">
					<PropField
						{doc}
						path={[...p, 'params', 'dialog']}
						label={m['creator.ono_text_box']()}
						type="select"
						options={[...dialogs.map((d) => ({ value: d.id, label: d.label })), ...(box && !dialogs.some((d) => d.id === box) ? [{ value: box, label: `${box} (?)` }] : [])]}
						placeholder={m['creator.ono_choose_box']()}
					/>
					{#if oncreatebox && !box}
						<button type="button" class="secondary" onclick={() => oncreatebox(i)} data-testid="ono-create-box">{m['creator.ono_create_box']()}</button>
					{/if}
				</div>
				<!-- how it comes in: one preset (owner 2026-10-03); every timing field under Advanced -->
				{@const ono = matchOnoPreset(event.params)}
				<div class="seg presets" role="group" aria-label={m['creator.ono_preset']()}>
					{#each ONO_PRESETS as preset (preset.id)}
						<button type="button" class="ghost" class:active={ono === preset.id} aria-pressed={ono === preset.id} onclick={() => setOnoPreset(i, preset)} data-testid="ono-preset-{preset.id}">{m[`creator.ono_preset_${preset.id}`]()}</button>
					{/each}
				</div>
				<details class="advanced">
					<summary>{m['creator.advanced']()}</summary>
					{#if box}<p class="hint">{m['creator.ono_box_hint']({ id: box })}</p>{/if}
					<div class="grid g3">
						<PropField {doc} path={[...p, 'params', 'effect']} label="effect" type="select" options={Object.values(OnomatopoeiaEffect)} placeholder="none" />
						<PropField {doc} path={[...p, 'params', 'duration']} label="duration ms" type="number" step={100} placeholder="2000" />
						<PropField {doc} path={[...p, 'params', 'delay']} label="delay ms" type="number" step={100} placeholder="0" />
						<PropField {doc} path={[...p, 'params', 'mode']} label="mode" type="select" options={Object.values(TriggerMode)} placeholder="poly" />
						<PropField {doc} path={[...p, 'params', 'wait']} label="wait (tap)" type="bool" placeholder="false" />
						<PropField
							{doc}
							path={[...p, 'params', 'stay']}
							label={m['creator.ono_after']()}
							type="select"
							options={[{ value: false, label: m['creator.ono_disappears']() }]}
							placeholder={m['creator.ono_stays']()}
						/>
					</div>
				</details>
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
						<button type="button" class="ghost" onclick={() => removeCharacter(i, ci)} aria-label={m['creator.remove']()} title={m['creator.remove']()}><CloseIcon /></button>
					</div>
				{/each}
				<button type="button" class="secondary" onclick={() => addCharacter(i)}>+ character</button>
			{:else}
				<p class="hint">No parameters.</p>
			{/if}

			{#if sliceEvents}
				<div class="grid g3 trigger">
					<PropField {doc} path={[...p, 'trigger', 'type']} label={m['creator.trigger']()} type="select" options={TRIGGER_OPTIONS} required />
					<PropField {doc} path={[...p, 'trigger', 'once']} label={m['creator.trigger_once']()} type="bool" placeholder="false" />
					<!-- area or the whole slice (no rect) – click, hover and intersect all take either (owner 2026-10-03) -->
					<label class="whole" title={m['creator.event_whole_slice_hint']()}>
						<input type="checkbox" checked={!event.trigger?.rect} onchange={(e) => setWholeSlice(i, e.currentTarget.checked)} data-testid="event-whole-{i}" />
						{m['creator.event_whole_slice']()}
					</label>
				</div>
			{/if}
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
	/* one line until opened: chevron, number, type, what it does (muted, cut), the trigger */
	.line {
		flex: 1;
		min-width: 0;
		justify-content: flex-start;
		gap: 0.5rem;
		padding: 0 0.25rem;
		text-align: left;
	}
	/* the whole line is the handle (open / drag): no ghost hover frame (owner 2026-10-03) */
	.line,
	.line:hover {
		border-color: transparent;
		background: none;
	}
	.line :global(svg) {
		flex: none;
		transition: transform var(--duration-base) var(--easing);
	}
	.event.open .line :global(svg) {
		transform: rotate(90deg);
	}
	.line .idx {
		flex: none;
		min-width: 1.5em;
	}
	.line .type {
		flex: none;
	}
	/* the speaker: a tiny avatar (its name when there is no image) */
	.line .avatar {
		flex: none;
		width: 1.25em;
		height: 1.25em;
		object-fit: cover;
		border-radius: var(--radius-full);
	}
	.line .who {
		flex: none;
	}
	.line .sum,
	.line .trig {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--text-muted);
	}
	/* the words keep their room; the trigger gives way first */
	.line .sum {
		flex: 1 1 auto;
		min-width: 3em;
	}
	.line .trig {
		flex: 0 4 auto;
	}
	.type-field {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		max-width: 16rem;
	}
	/* drag: the drop line above / below the target, picked ones framed, the moving ones faint */
	.event.over {
		box-shadow: inset 0 3px 0 var(--cyan);
	}
	.event.over.below {
		box-shadow: inset 0 -3px 0 var(--cyan);
	}
	.event.picked {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.event.moving {
		opacity: 0.5;
	}
	header[draggable='true'] {
		cursor: grab;
	}
	header {
		display: flex;
		gap: 0.4rem;
		align-items: center;
	}
	.idx {
		font-weight: var(--font-weight-regular);
	}
	.tools {
		margin-left: auto;
		display: flex;
		flex: none;
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
	.field label {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
	}
	.trigger {
		border-top: 1px dashed var(--border-muted);
		padding-top: 0.3rem;
	}
	/* presets: one segmented control, the matching one inverted */
	.presets {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(5.5rem, 1fr));
		border: var(--border-width-regular) solid var(--ink);
	}
	.advanced summary {
		cursor: pointer;
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.advanced[open] summary {
		margin-bottom: var(--space-05);
	}
	.whole {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		align-self: end;
		min-height: var(--control-height);
	}
	.add {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
	}
	.add button {
		padding: 0 var(--space-075);
	}
</style>
