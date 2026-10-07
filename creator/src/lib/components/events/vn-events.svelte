<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { CloseIcon, Icon, SortGrip, confirmDialog, sortable } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { characterImage, characterIndex, clone, getAt, resolveCharacterId, scriptLabels, Trigger, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { clickSelect, moveMany } from '$lib/editor/multi-select.ts';
	import { isKnownMissing } from '$lib/editor/characters.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { FORM_TYPES, castOptions, conditionWords, summary, template, triggerWord } from '$lib/editor/vn-forms.ts';
	import { listLanguage } from '$lib/editor/text-language.svelte.ts';
	import { exprContext } from '$lib/editor/expr-context.svelte.ts';
	import type { CharacterChoices } from '$lib/components/characters/character-ref-field.svelte';
	import EventMessage from '$lib/components/events/event-message.svelte';
	import EventAskText from '$lib/components/events/event-ask-text.svelte';
	import EventChoice from '$lib/components/events/event-choice.svelte';
	import EventFlow from '$lib/components/events/event-flow.svelte';
	import EventShowTextBox from '$lib/components/events/event-show-text-box.svelte';
	import EventCast from '$lib/components/events/event-cast.svelte';
	import EventCondition from '$lib/components/events/event-condition.svelte';
	import EventTrigger from '$lib/components/events/event-trigger.svelte';
	import GiveItemField from '$lib/components/fields/give-item-field.svelte';
	import ReachChip, { type ReachMark } from '$lib/components/chapter/reach-chip.svelte';

	/**
	 * Visual-novel events: group scripts (`group.events`) or slice-triggered events (`slice.events`, with `trigger`).
	 * The list and each row's chrome (one line until opened, multi-select + drag, the type, "Only if", the trigger); one
	 * form component per type (events/event-*.svelte, COMPONENTS.md step 8). No raw JSON (owner 2026-10-03) – another
	 * type is only named, to remove.
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
		oncreatebox,
		points: groupPoints,
		groupLabels = [],
		markOf = () => null
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
		 * text boxes a showTextBox event may reveal (`params.dialog`): the slice's boxes (slice events) or the boxes of
		 * the group's slices (a group script)
		 */
		dialogs?: { id: string; label: string }[];
		/** "+ New text box": a new text box on the slice, revealed by event `index` */
		oncreatebox?: (index: number) => void;
		/** the named points of the group's script (slice events divert into it); a group script uses its own */
		points?: string[];
		/** the chapter's group labels (a jump's group, the random list) */
		groupLabels?: string[];
		/** an event no run reaches, a test that is never true, a loop with no way out: a label (@scutoons/content reach.ts) */
		markOf?: (index: number) => ReachMark;
	} = $props();

	const events = $derived<any[]>(doc.get(doc.primary, eventsPath) ?? []);
	/** the summaries' language: the header's text language, else the source */
	const chain = $derived([listLanguage(doc.textLanguages, doc.textSource), doc.textSource]);
	/** the points a divert can go to: the group's script (a slice event diverts into it) or this script */
	const pointNames = $derived(groupPoints ?? scriptLabels(events));
	const charOptions = $derived(characters.map((c) => ({ value: c.name, label: c.label ? `${c.name} (${c.label})` : c.name })));
	const cast = $derived(castOptions(choices));
	const newEvent = (type: string) => template(type, { firstCharacter: characters[0]?.name, events, pointNames });

	function add(type: string) {
		const event = newEvent(type);
		// a new slice event covers the whole slice (when scrolled into view); an area is drawn with the ⚡ tool or switched on below
		if (sliceEvents) event.trigger = { type: Trigger.INTERSECT };
		doc.edit((file) => {
			let list = getAt(file, eventsPath);
			if (!Array.isArray(list)) {
				const parent = getAt(file, eventsPath.slice(0, -1));
				if (!parent) return;
				// read it back through the state proxy: the plain [] of the assignment is not the reactive list (a slice
				// without events showed nothing after "+ event")
				parent[eventsPath[eventsPath.length - 1]] = [];
				list = parent[eventsPath[eventsPath.length - 1]];
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
			((type === 'showMessage' || type === 'askText') && first('textarea')) ||
			((type === 'joinScene' || type === 'leaveScene') && first('[data-testid="character-ref-open"]')) ||
			[...root.querySelectorAll<HTMLElement>('input:not([type="checkbox"]), textarea, select, button[data-testid]')].find(
				(el) => !el.closest('.type-field') && !el.closest('header')
			);
		target?.focus();
		root.scrollIntoView({ block: 'nearest' });
	}

	function remove(index: number) {
		doc.edit((file) => {
			const list = getAt(file, eventsPath);
			if (Array.isArray(list)) list.splice(index, 1);
		});
	}

	async function changeType(index: number, type: string) {
		if (!(await confirmDialog({ title: m['creator.change_type_title'](), message: m['creator.change_type_confirm']({ index: index + 1, type }), confirmLabel: m['creator.change_type_label']() }))) return;
		doc.edit((file) => {
			const list = getAt(file, eventsPath);
			if (!Array.isArray(list)) return;
			const next = newEvent(type);
			if (list[index]?.trigger) next.trigger = list[index].trigger;
			list[index] = next;
		});
	}

	// ── one-liners (owner 2026-10-03): an event is a line (number, type, what it does) until opened; the selected one opens ──
	let open = $state<number[]>([]);
	$effect(() => {
		if (selected >= 0 && !untrack(() => open).includes(selected)) open = [...untrack(() => open), selected];
	});
	const isOpen = (i: number) => open.includes(i);
	// a jump to an expression's problem in one of these events (Problems panel): its form opens, the field takes the jump
	$effect(() => {
		const key = exprContext.focus?.key;
		const prefix = `${doc.entity}|${eventsPath.join('.')}.`;
		if (!key?.startsWith(prefix)) return;
		const index = Number.parseInt(key.slice(prefix.length), 10);
		if (Number.isInteger(index) && !untrack(() => open).includes(index)) open = [...untrack(() => open), index];
	});

	// ── multi-select + drag (like the chapter tree): click = one (opens / folds it), Shift = range, Ctrl / Cmd = add / remove;
	// the grip (the shared Sortable) moves the picked ones together ──
	let picked = $state<number[]>([]);
	let anchor = $state<number | null>(null);
	function clickRow(index: number, event: MouseEvent) {
		const next = clickSelect(picked, anchor, index, event);
		picked = next.selected;
		anchor = next.anchor;
		if (event.shiftKey || event.ctrlKey || event.metaKey) return;
		open = isOpen(index) ? open.filter((i) => i !== index) : [...open, index];
		onselect?.(index);
	}
	function moveEvents(from: number, to: number) {
		const moving = picked.length > 1 && picked.includes(from) ? [...picked] : [from];
		let map = new Map<number, number>();
		doc.edit((file) => {
			const list = getAt(file, eventsPath);
			if (!Array.isArray(list)) return;
			const result = moveMany(list, moving, from, to);
			map = result.map;
			list.splice(0, list.length, ...result.list);
		});
		const moved = (i: number) => map.get(i) ?? i;
		picked = moving.map(moved).sort((a, b) => a - b);
		open = open.map(moved);
		anchor = anchor === null ? null : moved(anchor);
		if (selected >= 0) onselect?.(moved(selected));
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
		const refs = event?.type === 'showMessage' || event?.type === 'askText' ? (event.params?.character ? [event.params] : []) : event?.type === 'joinScene' || event?.type === 'leaveScene' ? (event.params?.characters ?? []) : [];
		return refs.map((r: any) => ({ id: String(r?.character ?? r?.name ?? ''), src: avatarOf(r) })).filter((p: { id: string }) => p.id);
	}
</script>

<section class="vn-events" use:sortable={{ onmove: moveEvents }} data-testid="vn-events">
	{#each events as event, i (i)}
		{@const p = [...eventsPath, i]}
		{@const known = FORM_TYPES.includes(event?.type)}
		<article class="event" class:selected={i === selected} class:open={isOpen(i)} class:picked={picked.length > 1 && picked.includes(i)} data-sort-row data-testid="event-{i}">
			<header>
				<SortGrip label={m['creator.sort_move']({ name: `${i + 1} ${event?.type ?? ''}`.trim() })} title={m['creator.sort_hint']()} testid="event-grip-{i}" />
				<button type="button" class="ghost line" onclick={(e) => clickRow(i, e)} aria-expanded={isOpen(i)} data-testid="event-line-{i}">
					<Icon name="chevron-right" />
					<span class="idx">{i + 1}</span>
					<span class="type">{event?.type}</span>
					{#each people(event) as who, wi (wi)}
						{#if who.src}<img class="avatar" src={who.src} alt={who.id} title={who.id} />{:else}<span class="who">{who.id}</span>{/if}
					{/each}
					<span class="sum">{conditionWords(event?.if) ? `${m['creator.vn_if_short']({ test: conditionWords(event.if) })} · ` : ''}{summary(event, chain, dialogs)}</span>
					<ReachChip mark={markOf(i)} />
					{#if sliceEvents && event?.trigger}<span class="trig">{triggerWord(event)}{event.trigger.rect ? '' : ` · ${m['creator.event_whole_slice']()}`}</span>{/if}
				</button>
				<span class="tools">
					<button type="button" class="ghost" onclick={() => remove(i)} aria-label={m['creator.remove']()} title={m['creator.remove']()}><CloseIcon /></button>
				</span>
			</header>

			{#if isOpen(i)}
				<label class="type-field">
					<span class="sc-label">{m['creator.field_type']()}</span>
					<select value={event?.type} onchange={(e) => changeType(i, e.currentTarget.value)}>
						{#each FORM_TYPES as t}<option value={t}>{t}</option>{/each}
						{#if !known}<option value={event?.type}>{event?.type}</option>{/if}
					</select>
				</label>
				{#if !known}
					<p class="hint">{m['creator.event_unknown']({ type: String(event?.type) })}</p>
				{:else if event.type === 'showMessage'}
					<EventMessage {doc} path={p} {choices} {charOptions} {cast} />
				{:else if event.type === 'askText'}
					<EventAskText {doc} path={p} {event} {choices} {charOptions} />
				{:else if event.type === 'addChoice'}
					<EventChoice {doc} path={p} {event} index={i} {pointNames} {groupLabels} />
				{:else if event.type === 'jumpTo' || event.type === 'label' || event.type === 'divert' || event.type === 'setVariable'}
					<EventFlow {doc} path={p} {event} {pointNames} {groupLabels} />
				{:else if event.type === 'giveItem'}
					<div class="sc-fields" data-cols="2" data-density="panel"><GiveItemField {doc} path={[...p, 'params']} testid="event-{i}-give" /></div>
				{:else if event.type === 'takeItem'}
					<div class="sc-fields" data-cols="3" data-density="panel"><GiveItemField {doc} path={[...p, 'params']} take testid="event-{i}-take" /></div>
				{:else if event.type === 'showTextBox'}
					<EventShowTextBox {doc} path={p} {event} index={i} {dialogs} {oncreatebox} />
				{:else if event.type === 'joinScene' || event.type === 'leaveScene'}
					<EventCast {doc} path={p} {event} index={i} {choices} {charOptions} {cast} />
				{/if}

				<EventCondition {doc} path={p} {event} index={i} />
				{#if sliceEvents}
					<EventTrigger {doc} path={p} {event} index={i} {onselect} />
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
	/* picked ones (Shift / Ctrl) framed: a drag by one picked event's grip moves them all */
	.event.picked {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
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
	.add {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
	}
	.add button {
		padding: 0 var(--space-075);
	}
</style>
