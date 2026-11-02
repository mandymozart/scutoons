// The compiler's messages as the editor shows them: sorted into a problems list, per-file markers, the place a click
// jumps to. The API keeps five fields per message (file, line, column, level, message; ModuleCompiler::diagnostics).
import { fileOrder } from './files.ts';
import type { Diagnostic, ModuleFiles } from './types.ts';

export const isError = (d: Diagnostic): boolean => d.level === 'error';
export const isWarning = (d: Diagnostic): boolean => d.level === 'warning';

/** the answer's list, whatever it holds, as clean diagnostics (a broken entry is dropped) */
export function readDiagnostics(raw: unknown): Diagnostic[] {
	if (!Array.isArray(raw)) return [];
	const out: Diagnostic[] = [];
	for (const d of raw) {
		if (typeof d !== 'object' || d === null) continue;
		const r = d as Record<string, unknown>;
		out.push({
			file: typeof r.file === 'string' ? r.file : '',
			line: Number.isFinite(Number(r.line)) ? Math.max(0, Math.trunc(Number(r.line))) : 0,
			column: Number.isFinite(Number(r.column)) ? Math.max(0, Math.trunc(Number(r.column))) : 0,
			level: typeof r.level === 'string' && r.level !== '' ? r.level : 'error',
			message: typeof r.message === 'string' ? r.message : ''
		});
	}
	return out;
}

/** errors first, then warnings and the rest; inside a level by file (lib.rs first), line, column */
export function sortProblems(list: Diagnostic[], files: ModuleFiles): Diagnostic[] {
	const order = fileOrder(files);
	const rank = (d: Diagnostic) => (isError(d) ? 0 : isWarning(d) ? 1 : 2);
	const at = (d: Diagnostic) => {
		const i = order.indexOf(d.file);
		return i < 0 ? order.length : i;
	};
	return [...list].sort((a, b) => rank(a) - rank(b) || at(a) - at(b) || a.line - b.line || a.column - b.column);
}

export function counts(list: Diagnostic[]): { errors: number; warnings: number } {
	return { errors: list.filter(isError).length, warnings: list.filter(isWarning).length };
}

/** a marker in one file's editor (1-based line and column, as rustc counts) */
export interface Marker {
	line: number;
	column: number;
	level: 'error' | 'warning' | 'info';
	message: string;
}

/** the markers of one file – only messages that name a line */
export function markersFor(file: string, list: Diagnostic[]): Marker[] {
	return list
		.filter((d) => d.file === file && d.line > 0)
		.map((d) => ({ line: d.line, column: Math.max(1, d.column), level: isError(d) ? 'error' : isWarning(d) ? 'warning' : 'info', message: d.message }));
}

/** files with a marker, for the dots on the file tabs: the worst level per file */
export function worstByFile(list: Diagnostic[]): Record<string, 'error' | 'warning'> {
	const out: Record<string, 'error' | 'warning'> = {};
	for (const d of list) {
		if (!d.file) continue;
		if (isError(d)) out[d.file] = 'error';
		else if (isWarning(d) && out[d.file] !== 'error') out[d.file] = 'warning';
	}
	return out;
}

export interface JumpTarget {
	file: string;
	line: number;
	column: number;
}

/** where a click on a problem goes; null when it names no place or the file is gone */
export function jumpTarget(d: Diagnostic, files: ModuleFiles): JumpTarget | null {
	if (!d.file || d.line <= 0 || !(d.file in files)) return null;
	return { file: d.file, line: d.line, column: Math.max(1, d.column) };
}

/** the document offset of a 1-based line / column in `text`, clamped into the text (a line past the end = the end) */
export function offsetAt(text: string, line: number, column: number): number {
	let start = 0;
	for (let l = 1; l < line; l++) {
		const nl = text.indexOf('\n', start);
		if (nl < 0) return text.length;
		start = nl + 1;
	}
	const nl = text.indexOf('\n', start);
	const end = nl < 0 ? text.length : nl;
	return Math.min(end, start + Math.max(0, column - 1));
}

/** the range a marker underlines: the word at the place (at least one character, never past the line) */
export function rangeFor(text: string, line: number, column: number): { from: number; to: number } {
	const from = offsetAt(text, line, column);
	const nl = text.indexOf('\n', from);
	const lineEnd = nl < 0 ? text.length : nl;
	let to = from;
	while (to < lineEnd && /[A-Za-z0-9_]/.test(text[to])) to++;
	if (to === from) to = Math.min(lineEnd, from + 1);
	// a marker on an empty rest of the line sits on the last character before it
	if (to === from && from > 0 && text[from - 1] !== '\n') return { from: from - 1, to: from };
	return { from, to };
}

/** "lib.rs:12:5" */
export const placeOf = (d: Diagnostic): string => (d.file ? (d.line > 0 ? `${d.file}:${d.line}${d.column > 0 ? `:${d.column}` : ''}` : d.file) : '');
