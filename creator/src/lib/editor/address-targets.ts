// A VN event's target in the toon's address book (#477; decisions/deck.md [contract] [typeahead] [address]): the one
// typeahead (components/fields/address-field.svelte) filters the server's entries by the kinds the target may be and fills
// the event's existing fields from an entry's `name`, walking `parent` (a reference) up. An event's target is a reference:
// the creator never needs an entry's address (only what can receive a word has one). No reference is parsed here and no
// fallback is guessed: the server wrote every entry; this only filters, looks up by `parent` and copies names.
import type { AddressBook, AddressEntry, AddressKind } from '@scutoons/api';
import { pickContentText } from '@scutoons/content';

/** which field of the target object holds each level of an entry (kind → field name; several kinds may share one) */
export type TargetFields = Partial<Record<AddressKind, string>>;

/** What a field may pick and what a pick writes – the event document's format, as data. */
export interface TargetSpec {
	kinds: readonly AddressKind[];
	fields: TargetFields;
	/** the field that says which kind was picked, and what it holds per kind (a jump's `type`) */
	type?: { field: string; values: Partial<Record<AddressKind, string>> };
	/**
	 * Another toon in the same field (#477 slice D): an entry of `kind` that is not the book's own toon narrows the field
	 * to that toon's book (the entry's name is the toon); `kinds` are what may be picked in there; a pick of a kind in `into`
	 * writes the toon into `field`. The scope is the picked entry's own data – nothing is parsed.
	 */
	scope?: { kind: AddressKind; kinds: readonly AddressKind[]; field: string; into: readonly AddressKind[] };
}

/** the targets of the event forms: the kinds they take and the fields a pick fills (the event documents' format) */
export const TARGETS = {
	/**
	 * jumpTo, a choice's jump, a group's end (`JumpToTarget`): the entry's kind is the target type (a group is a chapter
	 * target with its `label`), its name the `value`; another toon's chapter or group keeps the toon in `toon` – a toon's
	 * page is `type: 'toon'` without it. XR has no entries (no catalog): an XR target is shown as it is.
	 */
	jump: {
		kinds: ['season', 'chapter', 'group', 'product', 'toon'],
		fields: { chapter: 'value', group: 'label', season: 'value', product: 'value', toon: 'value' },
		type: { field: 'type', values: { chapter: 'chapter', group: 'chapter', season: 'season', product: 'product', toon: 'toon' } },
		scope: { kind: 'toon', kinds: ['chapter', 'group', 'toon'], field: 'toon', into: ['chapter', 'group'] }
	},
	/** joinScene / leaveScene – a row of `params.characters` */
	cast: { kinds: ['character', 'persona', 'look'], fields: { character: 'character', persona: 'persona', look: 'look' } },
	/** unlockLocation, setAvatarLocation – `params` */
	location: { kinds: ['location'], fields: { location: 'location' } },
	/** migrate – `params` (a region of the world map) */
	region: { kinds: ['region'], fields: { region: 'region' } },
	/** giveItem / takeItem, a choice's give / take */
	item: { kinds: ['item'], fields: { item: 'item' } },
	/** setVariable – `params` */
	variable: { kinds: ['variable'], fields: { variable: 'variable' } }
} as const satisfies Record<string, TargetSpec>;

/** a book's entries by reference (made once per book) */
const indexes = new WeakMap<AddressBook, Map<string, AddressEntry>>();
export function indexOf(book: AddressBook): Map<string, AddressEntry> {
	let index = indexes.get(book);
	if (!index) indexes.set(book, (index = new Map(book.entries.map((e) => [e.ref, e]))));
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
 * name or the reference (case-insensitive); in the book's order (parents before their children)
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
		const hay = `${entryLabel(entry, locale, book.source)} ${entry.name} ${entry.ref}`.toLowerCase();
		if (!words.every((w) => hay.includes(w))) continue;
		out.push(entry);
		if (out.length >= limit) break;
	}
	return out;
}

/**
 * the field values a pick of `entry` writes: each mapped level's name from the chain (a field several kinds share takes
 * the one the chain has), undefined (= removed) for the levels it doesn't have
 */
export function fillValues(entry: AddressEntry, index: Map<string, AddressEntry>, fields: TargetFields): Record<string, string | undefined> {
	const chain = chainOf(entry, index);
	const out: Record<string, string | undefined> = {};
	for (const [kind, field] of Object.entries(fields) as [AddressKind, string][]) out[field] ??= chain.find((e) => e.kind === kind)?.name;
	return out;
}

/** every field a spec writes: the levels', the kind's (`type`), the scope's */
export function specFields(spec: TargetSpec): string[] {
	return [...new Set([...Object.values(fields(spec)), ...(spec.type ? [spec.type.field] : []), ...(spec.scope ? [spec.scope.field] : [])])];
}
const fields = (spec: TargetSpec) => spec.fields as Record<string, string>;

/**
 * Everything a pick of `entry` from `book` writes: the levels' names, the kind's value (`type`), and the toon the field is
 * narrowed to (`scope`: the picked toon's name) for the kinds that keep it – undefined removes a field
 */
export function pickValues(entry: AddressEntry, book: AddressBook, spec: TargetSpec, scope: string | null = null): Record<string, string | undefined> {
	const out = fillValues(entry, indexOf(book), spec.fields);
	if (spec.type) out[spec.type.field] = spec.type.values[entry.kind];
	if (spec.scope) out[spec.scope.field] = scope && spec.scope.into.includes(entry.kind) ? scope : undefined;
	return out;
}

/** does picking `entry` narrow the field to another toon's book (an entry of the scope's kind, not the book's own toon) */
export function opensScope(entry: AddressEntry, book: AddressBook, spec: TargetSpec): boolean {
	return !!spec.scope && entry.kind === spec.scope.kind && entry.name !== book.toon;
}

/** what may be picked inside a scope's book: its kinds, and of the scope's kind only that toon itself (no scope in a scope) */
export function inScope(entry: AddressEntry, book: AddressBook, spec: TargetSpec): boolean {
	return !!spec.scope && spec.scope.kinds.includes(entry.kind) && !opensScope(entry, book, spec);
}

/** the toon the target object is narrowed to now (its scope field), or null: this toon (`here`) or none */
export function scopeOf(target: Record<string, unknown> | null | undefined, spec: TargetSpec, here: string): string | null {
	const v = spec.scope ? target?.[spec.scope.field] : undefined;
	return typeof v === 'string' && v && v !== here ? v : null;
}

/** the target object's current values of `names` (empty = undefined) */
export function currentValues(target: Record<string, unknown> | null | undefined, names: TargetFields | string[]): Record<string, string | undefined> {
	const out: Record<string, string | undefined> = {};
	for (const field of Array.isArray(names) ? names : Object.values(names)) {
		const v = target?.[field];
		out[field] = v === undefined || v === null || v === '' ? undefined : String(v);
	}
	return out;
}

/**
 * the entry a target's fields name now in `book` (the scope's book when `scope` is set): the one whose pick would write
 * exactly these values; null when none does
 */
export function currentEntry(book: AddressBook, spec: TargetSpec, target: Record<string, unknown> | null | undefined, scope: string | null = null): AddressEntry | null {
	const now = currentValues(target, specFields(spec));
	if (Object.values(currentValues(target, spec.fields)).every((v) => v === undefined)) return null;
	if (spec.scope && !scope && now[spec.scope.field] === book.toon) now[spec.scope.field] = undefined; // "into this toon" = this toon
	const kinds = scope ? (spec.scope?.kinds ?? []) : spec.kinds;
	const same = (values: Record<string, string | undefined>) => Object.keys(now).every((f) => values[f] === now[f]);
	return book.entries.find((e) => kinds.includes(e.kind) && (!scope || inScope(e, book, spec)) && same(pickValues(e, book, spec, scope))) ?? null;
}

/** the names of the `child` entries under the `parent`-kind entry called `name` (a chapter's groups) */
export function childNames(book: AddressBook, parent: AddressKind, name: string, child: AddressKind): string[] {
	const up = book.entries.find((e) => e.kind === parent && e.name === name);
	return up ? book.entries.filter((e) => e.kind === child && e.parent === up.ref).map((e) => e.name) : [];
}
