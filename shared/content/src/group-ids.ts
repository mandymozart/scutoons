// A chapter group's id (`Group.id`): its address inside the chapter (`chapter:<n>.<id>`, RULES A14). Unique per chapter,
// slug-safe, editable: a rename rewrites every reference to it in the toon (`renameGroupId`).

/** what an id may look like: letters, digits, `-` and `_`, not starting with a sign (the schema's `@pattern` on `Group.id`) */
export const GROUP_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9_-]*$/;

export function isGroupId(value: unknown): value is string {
	return typeof value === 'string' && GROUP_ID_PATTERN.test(value);
}

/** why `id` can't be the id of the group at `index` (empty, not slug-safe, taken by another group), or null */
export function groupIdProblem(groups: readonly ({ id?: unknown } | null | undefined)[], index: number, id: string): string | null {
	if (!id) return 'a group needs an id';
	if (!GROUP_ID_PATTERN.test(id)) return `"${id}" isn't a valid id: letters, digits, - and _ only, starting with a letter or digit`;
	const other = groups.findIndex((g, i) => i !== index && g?.id === id);
	if (other >= 0) return `group ${other + 1} of this chapter is already called "${id}"`;
	return null;
}

/** a free id for a new group: `g<n>`, n = the group's position, counted up past ids already taken */
export function newGroupId(groups: readonly ({ id?: unknown } | null | undefined)[], position = groups.length + 1): string {
	const taken = new Set(groups.map((g) => g?.id));
	let n = Math.max(1, position);
	while (taken.has(`g${n}`)) n++;
	return `g${n}`;
}

/** text typed as an id, made slug-safe (spaces → `-`, other signs dropped); may come back empty */
export function slugGroupId(text: string): string {
	return text
		.normalize('NFKD')
		.replace(/[̀-ͯ]/g, '')
		.trim()
		.replace(/\s+/g, '-')
		.replace(/[^A-Za-z0-9_-]/g, '')
		.replace(/^[-_]+/, '');
}

/** one reference a rename rewrote: the chapter, the JSON path inside it, the field */
export interface GroupIdChange {
	chapter: number;
	path: (string | number)[];
	field: 'label' | 'anyOf';
}

export interface RenameGroupIdInput {
	/** the toon's chapters: each its number and its document (`groups`, anything that holds jump targets); changed in place */
	chapters: { number: number | string; doc: { groups?: unknown; [k: string]: unknown } }[];
	/** the chapter the renamed group belongs to */
	chapter: number;
	from: string;
	to: string;
	/** the toon: a jump target naming it explicitly counts as one of its own */
	toon?: string;
}

const isObj = (v: unknown): v is Record<string, any> => !!v && typeof v === 'object' && !Array.isArray(v);

/**
 * Renames a group's id everywhere the toon refers to it: the group itself and every jump target into that chapter
 * (`{ type: 'chapter', value: <n>, label | anyOf }` – VN `jumpTo`, a group's `end`, slice event areas, choices …),
 * wherever it sits in a chapter document. A target into another toon is left alone. Returns the references rewritten
 * (not the group's own id); throws when the new id is invalid or taken.
 */
export function renameGroupId({ chapters, chapter, from, to, toon }: RenameGroupIdInput): GroupIdChange[] {
	const home = chapters.find((c) => Number(c.number) === chapter);
	const groups = (Array.isArray(home?.doc?.groups) ? home.doc.groups : []) as { id?: unknown }[];
	const index = groups.findIndex((g) => g?.id === from);
	if (index < 0) throw new Error(`no group "${from}" in chapter ${chapter}`);
	if (from === to) return [];
	const problem = groupIdProblem(groups, index, to);
	if (problem) throw new Error(problem);
	groups[index].id = to;

	const changes: GroupIdChange[] = [];
	const intoHere = (t: Record<string, any>) =>
		t.type === 'chapter' && Number(t.value) === chapter && (t.toon === undefined || t.toon === null || t.toon === '' || t.toon === toon);
	const visit = (node: unknown, path: (string | number)[], number: number) => {
		if (Array.isArray(node)) return node.forEach((x, i) => visit(x, [...path, i], number));
		if (!isObj(node)) return;
		if (intoHere(node)) {
			if (node.label === from) {
				node.label = to;
				changes.push({ chapter: number, path, field: 'label' });
			}
			if (Array.isArray(node.anyOf) && node.anyOf.includes(from)) {
				node.anyOf = node.anyOf.map((l: unknown) => (l === from ? to : l));
				changes.push({ chapter: number, path, field: 'anyOf' });
			}
		}
		for (const [k, v] of Object.entries(node)) if (v && typeof v === 'object') visit(v, [...path, k], number);
	};
	for (const c of chapters) visit(c.doc, [], Number(c.number));
	return changes;
}

/** a change as a person reads it: "Chapter 2 · group 3 (scene-13) · event 4: jump target" */
export function describeGroupIdChange(change: GroupIdChange, chapters: RenameGroupIdInput['chapters']): string {
	const c = chapters.find((x) => Number(x.number) === change.chapter);
	const parts: string[] = [`Chapter ${change.chapter}`];
	const p = change.path;
	for (let i = 0; i < p.length; i++) {
		const key = p[i];
		const next = p[i + 1];
		if (key === 'groups' && typeof next === 'number') {
			const g = (c?.doc?.groups as { id?: string }[] | undefined)?.[next];
			parts.push(`group ${next + 1}${g?.id ? ` (${g.id})` : ''}`);
			i++;
		} else if (key === 'slices' && typeof next === 'number') {
			parts.push(`slice ${next + 1}`);
			i++;
		} else if (key === 'items' && typeof next === 'number') {
			parts.push(`item ${next + 1}`);
			i++;
		} else if (key === 'events' && typeof next === 'number') {
			parts.push(`event ${next + 1}`);
			i++;
		} else if (key === 'end') parts.push('end');
	}
	parts.push(change.field === 'anyOf' ? 'random jump' : 'jump target');
	return parts.join(' · ');
}
