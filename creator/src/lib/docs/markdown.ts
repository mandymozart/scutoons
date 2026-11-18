// The tutorials' source is Markdown (creator/docs/tutorials/<NN-phase>/<NN-slug>.md, a _group.md per phase): a front matter
// of `key: value` lines, then the sections “## Facts” (- Label: text), “## Steps” (1. text) and “## Note”. A file
// without steps is a template: listed in the index, its tutorial still to write.
import type { Case, Fact, Group, Often, Step, Where, Who } from './catalog.ts';

function split(text: string): { meta: Record<string, string>; body: string } {
	const m = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/.exec(text);
	if (!m) throw new Error('no front matter');
	const meta: Record<string, string> = {};
	for (const line of m[1].split(/\r?\n/)) {
		const at = line.indexOf(':');
		if (at > 0) meta[line.slice(0, at).trim()] = line.slice(at + 1).trim();
	}
	return { meta, body: m[2].replace(/<!--[\s\S]*?-->/g, '') };
}

function sections(body: string): Record<string, string[]> {
	const out: Record<string, string[]> = {};
	let at = '';
	for (const line of body.split(/\r?\n/)) {
		const head = /^##\s+(.+)$/.exec(line);
		if (head) out[(at = head[1].trim().toLowerCase())] = [];
		else if (at && line.trim()) out[at].push(line.trim());
	}
	return out;
}

const list = (v: string | undefined) =>
	(v ?? '')
		.split(',')
		.map((s) => s.trim())
		.filter(Boolean);

export function parseCase(text: string): Case {
	const { meta, body } = split(text);
	const s = sections(body);
	const steps: Step[] = (s.steps ?? []).flatMap((l) => {
		const m = /^\d+\.\s+(.+)$/.exec(l);
		return m ? [{ text: m[1] }] : [];
	});
	const facts: Fact[] = (s.facts ?? []).flatMap((l) => {
		const m = /^-\s+([^:]+):\s+(.+)$/.exec(l);
		return m ? [{ label: m[1].trim(), text: m[2] }] : [];
	});
	const item: Case = {
		slug: meta.slug,
		title: meta.title,
		summary: meta.summary,
		who: list(meta.who) as Who[],
		where: list(meta.where) as Where[],
		often: Number(meta.often) as Often
	};
	if (meta.partly === 'true') item.partly = true;
	if (facts.length) item.facts = facts;
	if (steps.length) item.steps = steps;
	if (s.note?.length) item.note = s.note.join(' ');
	return item;
}

/** files: path → text, as import.meta.glob gives them; folders and files are ordered by their NN- prefix */
export function parseGroups(files: Record<string, string>): Group[] {
	const byDir = new Map<string, { group?: string; cases: [string, string][] }>();
	for (const [path, text] of Object.entries(files)) {
		const parts = path.replace(/\\/g, '/').split('/');
		const dir = parts[parts.length - 2];
		const name = parts[parts.length - 1];
		const entry = byDir.get(dir) ?? { cases: [] };
		if (name === '_group.md') entry.group = text;
		else entry.cases.push([name, text]);
		byDir.set(dir, entry);
	}
	return [...byDir.entries()]
		.sort(([a], [b]) => a.localeCompare(b))
		.map(([dir, entry]) => {
			if (!entry.group) throw new Error(`${dir}: no _group.md`);
			const { meta, body } = split(entry.group);
			return {
				key: meta.key,
				title: meta.title,
				intro: body.trim(),
				cases: entry.cases.sort(([a], [b]) => a.localeCompare(b)).map(([, text]) => parseCase(text))
			};
		});
}
