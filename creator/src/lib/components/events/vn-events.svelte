<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { CloseIcon, Icon, SortGrip, confirmDialog, region as selectionRegion, sortable } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { characterImage, characterIndex, clone, getAt, resolveCharacterId, scriptLabels, Trigger, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { clickSelect, moveMany } from '$lib/editor/multi-select.ts';
	import { applyVnSettings, makeClip } from '$lib/editor/clip.ts';
	import { clipWhat, pastedMessage, useClipHandler } from '$lib/editor/clipboard.svelte.ts';
	import { isKnownMissing } from '$lib/editor/characters.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { FORM_TYPES, castOptions, conditionWords, summary, template, triggerWord } from '$lib/editor/vn-forms.ts';
	import { joinBefore, offStage } from '$lib/editor/vn-stage.ts';
	import { listLanguage } from '$lib/editor/text-language.svelte.ts';
	import { exprContext } from '$lib/editor/expr-context.svelte.ts';
	import type { CharacterChoices } from '$lib/components/characters/character-ref-field.svelte';
	import EventMessage from '$lib/components/events/event-message.svelte';
	import EventAskText from '$lib/components/events/event-ask-text.svelte';
	import EventChoice from '$lib/components/events/event-choice.svelte';
	import EventFlow from '$lib/components/events/event-flow.svelte';
	import EventShowTextBox from '$lib/components/events/event-show-text-box.svelte';
	import EventFx from '$lib/components/events/event-fx.svelte';
	import EventUnlockLocation from '$lib/components/events/event-unlock-location.svelte';
	import EventMigrate from '$lib/components/events/event-migrate.svelte';
	import EventCast from '$lib/components/events/event-cast.svelte';
	import EventCondition from '$lib/components/events/event-condition.svelte';
	import EventTrigger from '$lib/components/events/event-trigger.svelte';
	import GiveItemField from '$lib/components/fields/give-item-field.svelte';
	import ReachChip, { type ReachMark } from '$lib/components/chapter/reach-chip.svelte';

	/**
	 * Visual-novel events: group scripts (`group.events`) or slice-triggered events (`slice.events`, with `trigger`).
	 * The list and each row's chrome (one line until opened, multi-select + drag, the type, "Only if", the trigger); one
	 * form component per type (events/event-*.svelte, COMPONENTS.md step 8). No raw JSON (owner 2026-10-03) – another
	 * type is only named, to remove. Selection (DESIGN.md "Selection"): every picked event's line is `sc-selected`;
	 * the list is a region of its own (`region`, default 'events') – slice events pass null: they belong to the slice
	 * canvas's region (the canvas selection).
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
		markOf = () => null,
		region = 'events'
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
		/** the selection region (@scutoons/ui region); null: part of the surrounding one */
		region?: string | null;
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
			((type === 'joinScene' || type === 'leaveScene') && first('[data-testid="cast-character-input"]')) ||
			[...root.querySelectorAll<HTMLElement>('input:not([type="checkbox"]), textarea, select, button[data-testid]')].find(
				(el) => !el.closest('.type-field') && !el.closest('header')
			);
		target?.focus();
		root.scrollIntoView({ block: 'nearest' });
	}

	/** speakers not on stage here (the validator's stage walk over the group): the line is marked, the form says it + fixes it */
	const offStageAt = $derived(offStage(doc.get(doc.primary, []), eventsPath));
	function joinSpeaker(index: number) {
		let done = false;
		doc.edit((file) => (done = joinBefore(file, eventsPath, index)));
		if (!done) return;
		const shift = (i: number) => (i >= index ? i + 1 : i);
		open = open.map(shift);
		picked = picked.map(shift);
		anchor = anchor === null ? null : shift(anchor);
		if (selected >= 0) onselect?.(shift(selected));
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
	// the event selected elsewhere (the canvas, a new one): it alone is picked
	$effect(() => {
		if (selected >= 0 && !untrack(() => picked).includes(selected)) {
			picked = [selected];
			anchor = selected;
		}
	});
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

	// ── copy / cut / paste (BACKLOG 10.39 step 4): the picked events of a script (its own region; slice events belong to
	// the canvas region – region null); a paste goes after the last picked one (else at the end), Shift+V puts the params
	// + `if` of the copied event of the same type onto every picked one ──
	const pickedNow = () => (picked.length ? [...picked] : selected >= 0 ? [selected] : []).filter((i) => i < events.length).sort((a, b) => a - b);
	useClipHandler(() => region, {
		copy() {
			const entries = pickedNow().map((i) => events[i]).filter(Boolean);
			if (!entries.length) return null;
			const clip = makeClip('events', doc.toon, $state.snapshot(entries));
			return { clip, what: clipWhat(clip) };
		},
		remove() {
			const gone = pickedNow();
			if (!gone.length) return;
			doc.edit((file) => {
				const list = getAt(file, eventsPath);
				if (Array.isArray(list)) for (const i of [...gone].reverse()) list.splice(i, 1);
			});
			picked = [];
			open = [];
			anchor = null;
		},
		paste(clip) {
			if (clip.kind !== 'events' && clip.kind !== 'event') return null;
			const at = pickedNow().length ? Math.max(...pickedNow()) + 1 : events.length;
			const entries = clip.entries.map((e) => {
				const copy = clone(e);
				delete copy.id; // a fresh one (assignEventIds in the edit)
				if (!sliceEvents) delete copy.trigger; // a slice event's trigger means nothing in a script
				else if (!copy.trigger) copy.trigger = { type: Trigger.INTERSECT };
				return copy;
			});
			doc.edit((file) => {
				let list = getAt(file, eventsPath);
				if (!Array.isArray(list)) {
					const parent = getAt(file, eventsPath.slice(0, -1));
					if (!parent) return;
					parent[eventsPath[eventsPath.length - 1]] = [];
					list = parent[eventsPath[eventsPath.length - 1]];
				}
				list.splice(at, 0, ...entries);
			});
			picked = entries.map((_, n) => at + n);
			anchor = at;
			open = open.map((i) => (i >= at ? i + entries.length : i));
			return { message: pastedMessage(clip, doc.toon) };
		},
		pasteSettings(clip) {
			if (clip.kind !== 'events' && clip.kind !== 'event') return null;
			const targets = pickedNow();
			if (!targets.length) return { message: m['creator.clip_no_settings']() };
			let applied = 0;
			doc.edit((file) => {
				const list = getAt(file, eventsPath);
				if (Array.isArray(list)) for (const i of targets) if (list[i] && applyVnSettings(list[i], clip.entries)) applied++;
			});
			if (!applied) return { message: m['creator.clip_settings_kind']({ what: clipWhat(clip) }) };
			return { message: pastedMessage(clip, doc.toon, true, m['creator.clip_what_events']({ count: applied })) };
		}
	});

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

<section class="vn-events" use:sortable={{ onmove: moveEvents }} use:selectionRegion={region ? { name: region, initial: true } : null} data-testid="vn-events">
	{#each events as event, i (i)}
		{@const p = [...eventsPath, i]}
		{@const known = FORM_TYPES.includes(event?.type)}
		<article class="event" class:open={isOpen(i)} data-sort-row data-testid="event-{i}">
			<header class:sc-selected={i === selected || picked.includes(i)}>
				<SortGrip label={m['creator.sort_move']({ name: `${i + 1} ${event?.type ?? ''}`.trim() })} title={m['creator.sort_hint']()} testid="event-grip-{i}" />
				<button type="button" class="ghost line" onclick={(e) => clickRow(i, e)} aria-expanded={isOpen(i)} data-testid="event-line-{i}">
					<Icon name="chevron-right" />
					<span class="idx">{i + 1}</span>
					<span class="type">{event?.type}</span>
					{#each people(event) as who, wi (wi)}
						{#if who.src}<img class="avatar" src={who.src} alt={who.id} title={who.id} />{:else}<span class="who">{who.id}</span>{/if}
					{/each}
					<span class="sum">{conditionWords(event?.if) ? `${m['creator.vn_if_short']({ test: conditionWords(event.if) })} · ` : ''}{summary(event, chain, dialogs)}</span>
					{#if offStageAt.has(i)}<span class="off-stage" title={m['creator.event_off_stage']({ who: offStageAt.get(i) ?? '' })}><Icon name="alert-triangle" /></span>{/if}
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
				{:else if event.type === 'fx'}
					<EventFx {doc} path={p} {event} />
				{:else if event.type === 'unlockLocation'}
					<EventUnlockLocation {doc} path={p} />
				{:else if event.type === 'setAvatarLocation'}
					<EventUnlockLocation {doc} path={p} avatar />
				{:else if event.type === 'migrate'}
					<EventMigrate {doc} path={p} {event} {charOptions} />
				{:else if event.type === 'joinScene' || event.type === 'leaveScene'}
					<EventCast {doc} path={p} {event} index={i} {choices} {cast} />
				{/if}

				{#if offStageAt.has(i)}
					{@const who = offStageAt.get(i) ?? ''}
					<p class="warn" data-testid="event-{i}-off-stage">
						<span>{event.type === 'askText' ? m['creator.event_off_stage_ask']({ who }) : m['creator.event_off_stage']({ who })}</span>
						<button type="button" class="secondary" onclick={() => joinSpeaker(i)} data-testid="event-{i}-join">+ {m['creator.event_off_stage_join']()}</button>
					</p>
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
	/* a speaker who isn't on stage (vn-stage.ts): a mark in the line, the reason + the fix in the form */
	.line .off-stage {
		display: inline-flex;
		color: var(--state-warning);
	}
	.event.open .line .off-stage :global(svg) {
		transform: none;
	}
	.warn {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05);
		margin: 0;
		padding-left: var(--space-05);
		border-left: 0.5rem solid var(--state-warning);
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
