<script lang="ts">
	import type { AddressEntry, AddressKind } from '@scutoons/api';
	import { getAt, type Path } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { addressBooks } from '$lib/editor/address-book.svelte.ts';
	import { chainLabel, currentEntry, currentValues, fillValues, indexOf, matchEntries, type TargetFields } from '$lib/editor/address-targets.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A VN event's target: ONE typeahead over the toon's address book (#477; decisions/deck.md [typeahead]: "One type head
	 * would be enough"). Offers the entries of `kinds` (label in the creator's language, kind, "draft"); arrows, Enter,
	 * Escape and the pointer. A pick fills the target object's fields (`fields`: kind → field) from the entry's name and
	 * the names above it, in one edit; `onpick` adds to that edit. Without a book (offline, a failed fetch) every field is
	 * plain text. `free`: the field a typed text may go into as it is (a new variable, another toon's place).
	 */
	let {
		doc,
		path,
		toon = doc.toon,
		kinds,
		fields,
		label,
		free,
		accept,
		onpick,
		testid = 'address'
	}: {
		doc: ContentDoc;
		/** the target object (the jump target, a cast row, the event's params …) */
		path: Path;
		/** whose book (default: the document's toon) */
		toon?: string;
		kinds: readonly AddressKind[];
		fields: TargetFields;
		label: string;
		free?: string;
		accept?: (entry: AddressEntry) => boolean;
		/** more changes in the pick's edit (the target object, the picked entry or null for a typed text, the fields' values before) */
		onpick?: (target: any, entry: AddressEntry | null, before: Record<string, string | undefined>) => void;
		testid?: string;
	} = $props();

	const uid = $props.id();
	const held = $derived(addressBooks.of(toon));
	const book = $derived(held.book);
	const locale = $derived(getLocale());
	const target = $derived<Record<string, unknown> | undefined>(getAt(doc.data[doc.primary], path));
	const current = $derived(book ? currentEntry(book, kinds, fields, target) : null);
	const raw = $derived(
		Object.values(currentValues(target, fields))
			.filter((v) => v !== undefined)
			.join(' · ')
	);
	const shown = $derived(current && book ? chainLabel(current, indexOf(book), locale, book.source) : raw);

	let open = $state(false);
	let query = $state('');
	let active = $state(0);
	const options = $derived(open && book ? matchEntries(book, kinds, query, locale, accept) : []);
	const typed = $derived(free && query.trim() && !options.some((e) => e.name === query.trim()) ? query.trim() : '');
	const count = $derived(options.length + (typed ? 1 : 0));

	function write(values: Record<string, string | undefined>, entry: AddressEntry | null) {
		const before = currentValues(target, fields);
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
		if (entry && book) write(fillValues(entry, indexOf(book), fields), entry);
		else if (typed && free) write({ [free]: typed }, null);
		else return;
		close();
	}

	function start() {
		open = true;
		query = '';
		active = 0;
		void addressBooks.refresh(toon);
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
		}
	}

	$effect(() => {
		// keep the active option visible
		if (open) document.getElementById(`${uid}-${active}`)?.scrollIntoView({ block: 'nearest' });
	});

	const kindName = (kind: AddressKind) => m[`creator.address_kind_${kind}`]();
	const setField = (field: string, value: string) => write({ [field]: value.trim() || undefined }, null);
</script>

<div class="field" data-testid={testid}>
	<label for="{uid}-input">{label}</label>
	{#if !book && !held.loading}
		<!-- no book (offline, a failed fetch): the current values stay editable as text -->
		<div class="plain">
			{#each Object.entries(fields) as [kind, field], i (field)}
				<input id={i === 0 ? `${uid}-input` : undefined} type="text" value={target?.[field] ?? ''} placeholder={kindName(kind as AddressKind)} aria-label={kindName(kind as AddressKind)} autocomplete="off" spellcheck="false" onchange={(e) => setField(field, e.currentTarget.value)} data-testid="{testid}-{field}" />
			{/each}
		</div>
		<span class="note">{m['creator.address_offline']()}</span>
	{:else}
		<div class="combo">
			<input
				id="{uid}-input"
				type="text"
				role="combobox"
				aria-expanded={open}
				aria-controls="{uid}-list"
				aria-autocomplete="list"
				aria-activedescendant={open && count ? `${uid}-${active}` : undefined}
				class:unknown={!current && !!raw && !!book}
				title={current?.address ?? (raw && book ? m['creator.address_unknown']() : undefined)}
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
					{#each options as entry, i (entry.address)}
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<li id="{uid}-{i}" role="option" aria-selected={i === active} class:sc-selected={i === active} onmousedown={(e) => e.preventDefault()} onclick={() => choose(i)} onmousemove={() => (active = i)} data-testid="{testid}-option">
							<span class="label">{chainLabel(entry, indexOf(book!), locale, book!.source)}</span>
							<span class="meta">{kindName(entry.kind)}{#if entry.origin.state === 'draft'} · {m['creator.address_draft']()}{/if}</span>
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
	.combo {
		position: relative;
		display: flex;
		min-width: 0;
	}
	.combo input,
	.plain input {
		flex: 1;
		min-width: 0;
	}
	.plain {
		display: flex;
		gap: var(--space-025);
	}
	/* a value the book doesn't have (another toon's place, a removed one, an old id) */
	input.unknown {
		border-style: dashed;
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
		justify-content: space-between;
		align-items: center;
		gap: var(--space-1);
		min-height: var(--control-height);
		box-sizing: border-box;
		padding: 0 var(--space-05);
		cursor: pointer;
	}
	.label {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
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
