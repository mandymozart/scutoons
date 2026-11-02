// A module's files as the designer edits them (agents/research/RIG-EDITING.md §7): add / rename / remove with the
// server's rules (ModuleSource: `[a-z0-9_]+.rs`, a `lib.rs`, ≤ 32 files, 256 KB each, 512 KB together), the dirty flag
// against what was last saved, and the limits as problems BEFORE a request is sent. Pure functions on plain objects.
import type { ModuleFiles } from './types.ts';

export const MAIN_FILE = 'lib.rs';
export const FILE_PATTERN = /^[a-z0-9_]+\.rs$/;
export const LIMITS = { maxFiles: 32, maxFileBytes: 262144, maxTotalBytes: 524288 } as const;

export type NameProblem = 'bad_name' | 'exists' | 'main_fixed' | 'too_many';

/** a problem with the module as it stands – the panel turns the code into a sentence (messages.ts `problem.<code>`) */
export interface FileProblem {
	code: 'no_main' | 'too_many' | 'file_too_large' | 'total_too_large' | 'bad_name' | 'empty';
	file?: string;
	/** the limit and the actual value (bytes or files) */
	limit?: number;
	actual?: number;
}

const encoder = new TextEncoder();
export const byteLength = (text: string): number => encoder.encode(text).length;

/** lib.rs first, then the others by name */
export function fileOrder(files: ModuleFiles): string[] {
	return Object.keys(files).sort((a, b) => (a === MAIN_FILE ? -1 : b === MAIN_FILE ? 1 : a < b ? -1 : a > b ? 1 : 0));
}

export const totalBytes = (files: ModuleFiles): number => Object.values(files).reduce((n, t) => n + byteLength(t), 0);

/** the problems the server would answer with `422 invalid_module` – found before the request */
export function fileProblems(files: ModuleFiles): FileProblem[] {
	const names = Object.keys(files);
	const out: FileProblem[] = [];
	if (names.length === 0) return [{ code: 'empty' }];
	if (names.length > LIMITS.maxFiles) out.push({ code: 'too_many', limit: LIMITS.maxFiles, actual: names.length });
	if (!(MAIN_FILE in files)) out.push({ code: 'no_main', file: MAIN_FILE });
	let total = 0;
	for (const name of fileOrder(files)) {
		if (!FILE_PATTERN.test(name)) out.push({ code: 'bad_name', file: name });
		const bytes = byteLength(files[name]);
		total += bytes;
		if (bytes > LIMITS.maxFileBytes) out.push({ code: 'file_too_large', file: name, limit: LIMITS.maxFileBytes, actual: bytes });
	}
	if (total > LIMITS.maxTotalBytes) out.push({ code: 'total_too_large', limit: LIMITS.maxTotalBytes, actual: total });
	return out;
}

/** a file name the user typed: "osc" is "osc.rs" */
export function normalizeName(typed: string): string {
	const name = typed.trim().toLowerCase();
	return name === '' || name.endsWith('.rs') ? name : `${name}.rs`;
}

export type Edit = { ok: true; files: ModuleFiles; name: string } | { ok: false; problem: NameProblem };

/** a new, empty file */
export function addFile(files: ModuleFiles, typed: string): Edit {
	const name = normalizeName(typed);
	if (!FILE_PATTERN.test(name)) return { ok: false, problem: 'bad_name' };
	if (name in files) return { ok: false, problem: 'exists' };
	if (Object.keys(files).length >= LIMITS.maxFiles) return { ok: false, problem: 'too_many' };
	return { ok: true, files: { ...files, [name]: '' }, name };
}

/** the file keeps its text; lib.rs is the module's root and keeps its name */
export function renameFile(files: ModuleFiles, from: string, typed: string): Edit {
	if (!(from in files)) return { ok: false, problem: 'bad_name' };
	const name = normalizeName(typed);
	if (from === MAIN_FILE || name === MAIN_FILE) return { ok: false, problem: 'main_fixed' };
	if (!FILE_PATTERN.test(name)) return { ok: false, problem: 'bad_name' };
	if (name === from) return { ok: true, files, name };
	if (name in files) return { ok: false, problem: 'exists' };
	const out: ModuleFiles = {};
	for (const key of Object.keys(files)) out[key === from ? name : key] = files[key];
	return { ok: true, files: out, name };
}

/** lib.rs can't be removed */
export function removeFile(files: ModuleFiles, name: string): Edit {
	if (name === MAIN_FILE) return { ok: false, problem: 'main_fixed' };
	if (!(name in files)) return { ok: false, problem: 'bad_name' };
	const { [name]: _gone, ...rest } = files;
	return { ok: true, files: rest, name: MAIN_FILE };
}

export function setText(files: ModuleFiles, name: string, text: string): ModuleFiles {
	return name in files && files[name] === text ? files : { ...files, [name]: text };
}

/** edited since the last save / load (names and texts) */
export function isDirty(saved: ModuleFiles, files: ModuleFiles): boolean {
	const a = Object.keys(saved);
	const b = Object.keys(files);
	return a.length !== b.length || b.some((k) => saved[k] !== files[k]);
}

/** a free name for "+ file": module.rs, module_2.rs … */
export function freshName(files: ModuleFiles, stem = 'module'): string {
	for (let i = 1; i < 1000; i++) {
		const name = i === 1 ? `${stem}.rs` : `${stem}_${i}.rs`;
		if (!(name in files)) return name;
	}
	return `${stem}_x.rs`;
}

/** the file to show after the active one went away */
export function neighbour(files: ModuleFiles, active: string): string {
	return active in files ? active : MAIN_FILE in files ? MAIN_FILE : (fileOrder(files)[0] ?? MAIN_FILE);
}
