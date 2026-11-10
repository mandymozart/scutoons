<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { AddressEntry, AddressKind } from '@scutoons/api';
	import { getAt, type Path } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { addressBooks } from '$lib/editor/address-book.svelte.ts';
	import { chainLabel, currentEntry, currentValues, entryLabel, indexOf, inScope, matchEntries, opensScope, pickValues, scopeOf, specFields, type TargetSpec } from '$lib/editor/address-targets.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A VN event's target: ONE typeahead over the toon's address book (#477; decisions/deck.md [typeahead]: "One type head
	 * would be enough"). Offers the entries of `spec.kinds` (label in the creator's language, kind, "draft"); arrows, Enter,
	 * Escape and the pointer. A pick fills the target object's fields (`spec`: kind → field, the kind's own field, the scope's)
	 * from the entry's name and the names above it, in one edit; `onpick` adds to that edit. Another toon (`spec.scope`):
	 * picking it narrows the field to that toon's book, shown as a chip; Backspace on the empty field (or the chip) leaves
	 * it. Without a book (offline, a failed fetch) every field is plain text. `free`: the field a typed text may go into as it
	 * is (a new variable, another toon's place). `legacy`: an old field shown while the mapped ones are empty (a cast row's
	 * `name`). `lead`: drawn before the text (a thumbnail); `children`: under it (a warning); `icon`: an option's picture.
	 */
	let { doc, path, toon = doc.toon, spec, label, free, legacy, accept, onpick, icon, lead, children, testid = 'address' }: {
		doc: ContentDoc;
		/** the target object (the jump target, a cast row, the event's params …) */
		path: Path;
		/** whose book (default: the document's toon) */
		toon?: string;
		spec: TargetSpec;
		label: string;
		free?: string;
		legacy?: string;
		accept?: (entry: AddressEntry) => boolean;
		/** more changes in the pick's edit (the target object, the picked entry or null for a typed text, the fields' values before) */
		onpick?: (target: any, entry: AddressEntry | null, before: Record<string, string | undefined>) => void;
		/** an option's picture (an asset path) */
		icon?: (entry: AddressEntry) => string | undefined;
		lead?: Snippet;
		children?: Snippet;
		testid?: string;
	} = $props();

	const uid = $props.id();
	const home = $derived(addressBooks.of(toon));
	const locale = $derived(getLocale());
	const target = $derived<Record<string, unknown> | undefined>(getAt(doc.data[doc.primary], path));

	let open = $state(false);
	let query = $state('');
	let active = $state(0);
	/** while open: the toon the field is narrowed to (null = this toon's book) */
	let picked = $state<string | null>(null);
	/** the toon whose book the field shows: the picked one while open, else the one the target is in */
	const scope = $derived(open ? picked : scopeOf(target, spec, toon));
	const held = $derived(scope ? addressBooks.of(scope) : home);
	const book = $derived(held.book);
	const scopeEntry = $derived(scope ? home.book?.entries.find((e) => e.kind === spec.scope?.kind && e.name === scope) : undefined);
	const scopeLabel = $derived(scope ? (scopeEntry && home.book ? entryLabel(scopeEntry, locale, home.book.source) : scope) : '');

	const current = $derived(book ? currentEntry(book, spec, target, scope) : null);
	const raw = $derived.by(() => {
		const values = Object.values(currentValues(target, spec.fields)).filter((v) => v !== undefined);
		if (!values.length) return legacy && target?.[legacy] ? String(target[legacy]) : '';
		const kind = spec.type ? target?.[spec.type.field] : undefined;
		return [...new Set([...(kind ? [String(kind)] : []), ...values])].join(' · ');
	});
	const shown = $derived(current && book ? chainLabel(current, indexOf(book), locale, book.source) : raw);

	const options = $derived.by(() => {
		if (!open || !book) return [];
		if (scope) return matchEntries(book, spec.scope?.kinds ?? [], query, locale, (e) => inScope(e, book, spec) && (!accept || accept(e)));
		return matchEntries(book, spec.kinds, query, locale, accept);
	});
	const typed = $derived(free && query.trim() && !options.some((e) => e.name === query.trim()) ? query.trim() : '');
	const count = $derived(options.length + (typed ? 1 : 0));

	function write(values: Record<string, string | undefined>, entry: AddressEntry | null) {
		const before = currentValues(target, specFields(spec));
		doc.edit((file) => {
			const t = getAt(file, path);
			if (!t) return;
			for (const [field, value] of Object.entries(values)) {
				if (value === undefined) delete t[field];
				else t[field] = value;
			}
			onpick?.(t, entry, before);
		}, `${path.join('.')}.address`);
	}

	function choose(i: number) {
		const entry = options[i];
		if (entry && book && !scope && opensScope(entry, book, spec)) return narrow(entry.name);
		if (entry && book) write(pickValues(entry, book, spec, scope), entry);
		else if (typed && free) write({ [free]: typed }, null);
		else return;
		close();
	}

	/** into another toon's book (its own fetch, the same cache rule); typing goes on in there */
	function narrow(into: string | null) {
		picked = into;
		query = '';
		active = 0;
		if (into) void addressBooks.refresh(into);
	}

	function start() {
		open = true;
		query = '';
		active = 0;
		picked = scopeOf(target, spec, toon);
		void addressBooks.refresh(picked ?? toon);
	}
	function close() {
		open = false;
		query = '';
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault();
			if (!open) return start();
			active = count ? (active + (event.key === 'ArrowDown' ? 1 : count - 1)) % count : 0;
		} else if (event.key === 'Enter' && open) {
			event.preventDefault();
			choose(active);
		} else if (event.key === 'Escape' && open) {
			event.preventDefault();
			event.stopPropagation();
			close();
		} else if (event.key === 'Backspace' && open && !query && picked) {
			event.preventDefault();
			narrow(null);
		}
	}

	$effect(() => {
		// keep the active option visible
		if (open) document.getElementById(`${uid}-${active}`)?.scrollIntoView({ block: 'nearest' });
	});

	const kindName = (kind: AddressKind) => m[`creator.address_kind_${kind}`]();
	const setField = (field: string, value: string) => write({ [field]: value.trim() || undefined }, null);
	/** the plain fields without a book: one per field (kinds may share one) */
	const plain = $derived([...new Map((Object.entries(spec.fields) as [AddressKind, string][]).map(([kind, field]) => [field, kind])).entries()]);
</script>

<div class="field" data-testid={testid}>
	<label for="{uid}-input">{label}</label>
	{#if !home.book && !home.loading}
		<!-- no book (offline, a failed fetch): the current values stay editable as text -->
		<div class="plain">
			{#each plain as [field, kind], i (field)}
				<input id={i === 0 ? `${uid}-input` : undefined} type="text" value={target?.[field] ?? ''} placeholder={kindName(kind)} aria-label={kindName(kind)} autocomplete="off" spellcheck="false" onchange={(e) => setField(field, e.currentTarget.value)} data-testid="{testid}-{field}" />
			{/each}
		</div>
		<span class="note">{m['creator.address_offline']()}</span>
	{:else}
		<div class="combo" class:unknown={!current && !!raw && !!book}>
			{@render lead?.()}
			{#if scope}
				<!-- the toon the field is narrowed to (#477 slice D): a click or Backspace on the empty field leaves it -->
				<button type="button" class="chip" tabindex="-1" title={m['creator.address_scope_leave']()} onmousedown={(e) => e.preventDefault()} onclick={() => (open ? narrow(null) : start())} data-testid="{testid}-scope">{scopeLabel} ›</button>
			{/if}
			<input
				id="{uid}-input"
				type="text"
				role="combobox"
				aria-expanded={open}
				aria-controls="{uid}-list"
				aria-autocomplete="list"
				aria-activedescendant={open && count ? `${uid}-${active}` : undefined}
				title={current?.ref ?? (raw && book ? m['creator.address_unknown']() : undefined)}
				value={open ? query : shown}
				placeholder={open && shown ? shown : m['creator.address_search']()}
				autocomplete="off"
				spellcheck="false"
				onfocus={start}
				onclick={() => !open && start()}
				onblur={close}
				oninput={(e) => {
					if (!open) start();
					query = e.currentTarget.value;
					active = 0;
				}}
				{onkeydown}
				data-testid="{testid}-input"
			/>
			{#if open}
				<ul id="{uid}-list" role="listbox" aria-label={label}>
					<!-- the keys are the combobox's (aria-activedescendant): an option is only clicked -->
					{#each options as entry, i (entry.ref)}
						{@const src = icon?.(entry)}
						{@const enters = !scope && opensScope(entry, book!, spec)}
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<li id="{uid}-{i}" role="option" aria-selected={i === active} class:sc-selected={i === active} onmousedown={(e) => e.preventDefault()} onclick={() => choose(i)} onmousemove={() => (active = i)} data-testid="{testid}-option">
							{#if src}<img src={asset(src)} alt="" onerror={(e) => ((e.currentTarget as HTMLImageElement).hidden = true)} />{/if}
							<span class="label">{chainLabel(entry, indexOf(book!), locale, book!.source)}{enters ? ' ›' : ''}</span>
							<span class="meta">{enters ? m['creator.address_scope_enter']() : kindName(entry.kind)}{#if entry.origin.state === 'draft'} · {m['creator.address_draft']()}{/if}</span>
						</li>
					{/each}
					{#if typed}
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<li id="{uid}-{options.length}" role="option" aria-selected={active === options.length} class:sc-selected={active === options.length} onmousedown={(e) => e.preventDefault()} onclick={() => choose(options.length)} onmousemove={() => (active = options.length)} data-testid="{testid}-typed">
							{m['creator.address_as_typed']({ text: typed })}
						</li>
					{/if}
					{#if !count}<li class="note" role="presentation">{held.loading && !book ? m['creator.loading']() : m['creator.address_none']()}</li>{/if}
				</ul>
			{/if}
		</div>
		{#if held.error}<span class="note">{m['creator.address_stale']()}</span>{/if}
	{/if}
	{@render children?.()}
</div>

<style>
	/* the same look as prop-field.svelte */
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
	}
	label {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
	}
	.combo,
	.plain {
		position: relative;
		display: flex;
		align-items: center;
		gap: var(--space-025);
		min-width: 0;
	}
	.combo input,
	.plain input {
		flex: 1;
		min-width: 0;
	}
	/* a value the book doesn't have (another toon's place, a removed one, an old id) */
	.combo.unknown input {
		border-style: dashed;
	}
	.chip,
	.label {
		flex: none;
		max-width: 40%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	ul {
		position: absolute;
		top: 100%;
		left: 0;
		right: 0;
		z-index: 30;
		list-style: none;
		margin: 0;
		padding: 0;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		max-height: 18rem;
		overflow: auto;
	}
	li[role='option'] {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		min-height: var(--control-height);
		box-sizing: border-box;
		padding: 0 var(--space-05);
		cursor: pointer;
	}
	li img {
		flex: none;
		width: 1.5rem;
		height: 1.5rem;
		object-fit: contain;
	}
	.label {
		flex: 1;
		max-width: none;
		min-width: 0;
	}
	.meta,
	.note {
		flex: none;
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
	li.note {
		padding: var(--space-025) var(--space-05);
	}
</style>
