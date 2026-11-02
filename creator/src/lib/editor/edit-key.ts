// The editor's address (agents/research/EDITOR.md §4, owner 2026-11-02): `/edit/<kind>:<id>?w=<workspace>&sel=<path>`.
// The key is the document being edited – the subject; the workspace is how it is shown; the selection is what is picked in
// it. Pure: the route reads the key and the query through these, so the rules are tested (test/edit-key.test.ts).
import { isModuleId } from '@scutoons/rig-designer/store';

export type Workspace = 'sound';

/** what a key edits: a rig (a character's, later an item's) or a module's sources */
export type EditTarget =
	| { kind: 'rig'; key: string; subject: string; character: string }
	| { kind: 'module'; key: string; id: string };

const CHARACTER = /^character\.([a-z0-9][a-z0-9_-]*)(?:\.[a-z0-9][a-z0-9_-]*)?$/;

/** `rig:character.catpope` · `rig:character.catpope.dj` · `module:walk`; anything else (an unknown kind, a bad id) → null */
export function parseEditKey(key: string): EditTarget | null {
	const cut = key.indexOf(':');
	if (cut < 1) return null;
	const kind = key.slice(0, cut);
	const id = key.slice(cut + 1);
	if (kind === 'module' && isModuleId(id)) return { kind: 'module', key, id };
	if (kind === 'rig') {
		const c = CHARACTER.exec(id);
		if (c) return { kind: 'rig', key, subject: id, character: c[1] };
	}
	return null;
}

/** the workspaces a target has; the first is the default (`?w=` absent or unknown) – `rig` and `module` → sound */
export function workspacesOf(_target: EditTarget): Workspace[] {
	return ['sound'];
}

export function workspaceOf(target: EditTarget, w: string | null): Workspace {
	const all = workspacesOf(target);
	return all.find((x) => x === w) ?? all[0];
}

/** the module the Rust tab shows: a module target's own, or a rig's `?sel=module:<id>` (none: null) */
export function selectedModule(target: EditTarget, sel: string | null): string | null {
	if (target.kind === 'module') return target.id;
	const s = sel ? parseEditKey(sel) : null;
	return s?.kind === 'module' ? s.id : null;
}

/** the address of a target in a workspace, optionally with a selection */
export function editHref(key: string, options: { w?: Workspace; sel?: string | null } = {}): string {
	const q = new URLSearchParams();
	if (options.w) q.set('w', options.w);
	if (options.sel) q.set('sel', options.sel);
	const query = q.toString();
	return `/edit/${key}${query ? `?${query}` : ''}`;
}
