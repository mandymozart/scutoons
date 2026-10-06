/**
 * Hidden stable event ids (agents/EXPRESSIONS.md §5): every VN event of a chapter / scene – a group's script, a slice's
 * triggered events, a placed item's tap events – carries an `id` the reader's save game remembers it by (repeat rules,
 * `resolved(id)` / `count(id)`). The author never types one: an event without an id (a new one, a pasted or imported
 * one, or one stored before step 3) gets a fresh one; a second event with the same id in the document (a copy) gets a
 * fresh one too. Random (`e` + 7 letters / digits), so a chapter and its scenes – separate documents – don't collide;
 * validation reports a duplicate across the chapter (validate.ts `checkEventIds`).
 */
const ALPHABET = 'abcdefghijklmnopqrstuvwxyz0123456789';

export function newEventId(): string {
	const bytes = new Uint8Array(7);
	crypto.getRandomValues(bytes);
	return `e${[...bytes].map((b) => ALPHABET[b % ALPHABET.length]).join('')}`;
}

type Obj = Record<string, any>;
const isObj = (v: unknown): v is Obj => !!v && typeof v === 'object' && !Array.isArray(v);

/** every event list of a group (inline in a chapter, or a scene document) */
function groupEventLists(group: unknown): Obj[][] {
	if (!isObj(group) || group.ref) return [];
	const lists: unknown[] = [group.events];
	for (const slice of Array.isArray(group.slices) ? group.slices : []) {
		if (!isObj(slice)) continue;
		lists.push(slice.events);
		for (const item of Array.isArray(slice.items) ? slice.items : []) if (isObj(item)) lists.push(item.events);
	}
	return lists.filter((l): l is Obj[] => Array.isArray(l));
}

/**
 * Gives every event of a chapter (`groups`) or scene (one group) document an id – in place; the number of events that
 * got one. `taken`: ids used elsewhere (they stay with their owner).
 */
export function assignEventIds(doc: unknown, kind: 'chapter' | 'group', taken: Iterable<string> = []): number {
	if (!isObj(doc)) return 0;
	const groups = kind === 'chapter' ? (Array.isArray(doc.groups) ? doc.groups : []) : [doc];
	const used = new Set<string>(taken);
	let assigned = 0;
	for (const group of groups)
		for (const list of groupEventLists(group))
			for (const event of list) {
				if (!isObj(event)) continue;
				if (typeof event.id === 'string' && event.id && !used.has(event.id)) {
					used.add(event.id);
					continue;
				}
				let id = newEventId();
				while (used.has(id)) id = newEventId();
				event.id = id;
				used.add(id);
				assigned++;
			}
	return assigned;
}
