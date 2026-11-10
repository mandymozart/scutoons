// A VN event's target in the toon's address book (#477 slice B; decisions/deck.md [contract] [typeahead]): the one
// typeahead (components/fields/address-field.svelte) filters the server's entries by the kinds the target may be and fills
// the event's existing fields from an entry's `name`, walking `parent` up. No address is parsed here and no fallback is
// guessed: the server wrote every entry; this only filters, looks up by `parent` and copies names.
import type { AddressBook, AddressEntry, AddressKind } from '@scutoons/api';
import { pickContentText } from '@scutoons/content';

/** which field of the target object holds each level of an entry (kind → field name) */
export type TargetFields = Partial<Record<AddressKind, string>>;

/** the targets of the event forms: the kinds they take and the fields a pick fills (the event documents' format) */
export const TARGETS = {
	/** jumpTo, a choice's jump, a group's end – a chapter target (`value`, `label`) */
	jump: { kinds: ['chapter', 'group'], fields: { chapter: 'value', group: 'label' } },
	/** joinScene / leaveScene – a row of `params.characters` */
	cast: { kinds: ['character', 'persona', 'look'], fields: { character: 'character', persona: 'persona', look: 'look' } },
	/** unlockLocation, setAvatarLocation – `params` */
	location: { kinds: ['location'], fields: { location: 'location' } },
	/** giveItem / takeItem, a choice's give / take */
	item: { kinds: ['item'], fields: { item: 'item' } },
	/** setVariable – `params` */
	variable: { kinds: ['variable'], fields: { variable: 'variable' } }
} as const satisfies Record<string, { kinds: readonly AddressKind[]; fields: TargetFields }>;

/** a book's entries by address (made once per book) */
const indexes = new WeakMap<AddressBook, Map<string, AddressEntry>>();
export function indexOf(book: AddressBook): Map<string, AddressEntry> {
	let index = indexes.get(book);
	if (!index) indexes.set(book, (index = new Map(book.entries.map((e) => [e.address, e]))));
	return index;
}

/** an entry and the entries above it, the entry first */
export function chainOf(entry: AddressEntry, index: Map<string, AddressEntry>): AddressEntry[] {
	const chain = [entry];
	for (let up = entry.parent ? index.get(entry.parent) : undefined; up && !chain.includes(up); up = up.parent ? index.get(up.parent) : undefined) chain.push(up);
	return chain;
}

/** what an entry is called in `locale` (the one locale order, generated); its name when it has no label */
export function entryLabel(entry: AddressEntry, locale: string, source: string): string {
	return pickContentText(entry.labels, locale, source) ?? entry.name;
}

/** the entry's label with the ones above it: "Cat Pope › Mensch › Helm" */
export function chainLabel(entry: AddressEntry, index: Map<string, AddressEntry>, locale: string, source: string): string {
	return chainOf(entry, index)
		.reverse()
		.map((e) => entryLabel(e, locale, source))
		.join(' › ');
}

/**
 * The entries a target may be: of `kinds`, `accept`ed, every word of `query` in the label (the creator's locale), the
 * name or the address (case-insensitive); in the book's order (parents before their children)
 */
export function matchEntries(
	book: AddressBook,
	kinds: readonly AddressKind[],
	query: string,
	locale: string,
	accept?: (entry: AddressEntry) => boolean,
	limit = 50
): AddressEntry[] {
	const words = query.toLowerCase().split(/\s+/).filter(Boolean);
	const out: AddressEntry[] = [];
	for (const entry of book.entries) {
		if (!kinds.includes(entry.kind) || (accept && !accept(entry))) continue;
		const hay = `${entryLabel(entry, locale, book.source)} ${entry.name} ${entry.address}`.toLowerCase();
		if (!words.every((w) => hay.includes(w))) continue;
		out.push(entry);
		if (out.length >= limit) break;
	}
	return out;
}

/** the field values a pick of `entry` writes: each mapped level's name from the chain, undefined (= removed) for the levels it doesn't have */
export function fillValues(entry: AddressEntry, index: Map<string, AddressEntry>, fields: TargetFields): Record<string, string | undefined> {
	const chain = chainOf(entry, index);
	const out: Record<string, string | undefined> = {};
	for (const [kind, field] of Object.entries(fields) as [AddressKind, string][]) out[field] = chain.find((e) => e.kind === kind)?.name;
	return out;
}

/** the target object's current values of the mapped fields (empty = undefined) */
export function currentValues(target: Record<string, unknown> | null | undefined, fields: TargetFields): Record<string, string | undefined> {
	const out: Record<string, string | undefined> = {};
	for (const field of Object.values(fields)) {
		const v = target?.[field];
		out[field] = v === undefined || v === null || v === '' ? undefined : String(v);
	}
	return out;
}

/** the entry a target's fields name now: the one whose pick would write exactly these values; null when none does */
export function currentEntry(book: AddressBook, kinds: readonly AddressKind[], fields: TargetFields, target: Record<string, unknown> | null | undefined): AddressEntry | null {
	const now = currentValues(target, fields);
	if (Object.values(now).every((v) => v === undefined)) return null;
	const index = indexOf(book);
	const same = (values: Record<string, string | undefined>) => Object.keys(now).every((f) => values[f] === now[f]);
	return book.entries.find((e) => kinds.includes(e.kind) && same(fillValues(e, index, fields))) ?? null;
}

/** the names of the `child` entries under the `parent`-kind entry called `name` (a chapter's groups) */
export function childNames(book: AddressBook, parent: AddressKind, name: string, child: AddressKind): string[] {
	const up = book.entries.find((e) => e.kind === parent && e.name === name);
	return up ? book.entries.filter((e) => e.kind === child && e.parent === up.address).map((e) => e.name) : [];
}
