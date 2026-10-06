// The analyser's problems in the creator's language (agents/EXPRESSIONS.md step 4): ONE message map – `expr:<code>` →
// `creator.expr_<code>` in messages/{de,en}.json, filled from the problem's `args` (+ "did you mean" and the position).
// The text fields' tooltips, the builder and the Problems panel word expression problems through it.
import type { ListKind } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';

type Fill = Record<string, string>;
const messages = m as unknown as Record<string, ((inputs?: Fill) => string) | undefined>;

/** "1 argument" / "at least 2 arguments" (the analyser's English) in the creator's language */
function argumentCount(english: string | undefined): string {
	const match = /^(at least )?(\d+) argument/.exec(english ?? '');
	if (!match) return english ?? '';
	const count = match[2];
	const words = count === '1' ? m['creator.expr_args_one']() : m['creator.expr_args_many']({ count });
	return match[1] ? m['creator.expr_args_at_least']({ count: words }) : words;
}

const TYPES: Record<string, () => string> = {
	number: () => m['creator.expr_type_number'](),
	string: () => m['creator.expr_type_string'](),
	boolean: () => m['creator.expr_type_boolean'](),
	any: () => m['creator.expr_type_any']()
};

/** a problem (code without the `expr:` prefix) in words: "unknown name `map.totl` – did you mean `map.total`?" */
export function exprText(p: { code: string; args?: string[]; suggestion?: string; message?: string }): string {
	const code = p.code.replace(/^expr:/, '');
	const args = [...(p.args ?? [])];
	if (code === 'argument-count') args[1] = argumentCount(args[1]);
	if (code === 'argument-type') args[1] = TYPES[args[1]]?.() ?? args[1];
	const fn = messages[`creator.expr_${code.replace(/-/g, '_')}`];
	const text = fn ? fn({ a: args[0] ?? '', b: args[1] ?? '', c: args[2] ?? '' }) : (p.message ?? code);
	return p.suggestion ? m['creator.expr_did_you_mean']({ text, suggestion: p.suggestion }) : text;
}

/** with its place in the text (1-based characters): "… – at 2–9" */
export function exprTextAt(p: { code: string; args?: string[]; suggestion?: string; message?: string; start: number; end: number }): string {
	const position = `${p.start + 1}${p.end > p.start + 1 ? `–${p.end}` : ''}`;
	return m['creator.expr_at']({ message: exprText(p), position });
}

const LIST_WORDS: Record<ListKind, () => string> = {
	sequence: () => m['creator.expr_list_sequence'](),
	cycle: () => m['creator.expr_list_cycle'](),
	shuffle: () => m['creator.expr_list_shuffle'](),
	once: () => m['creator.expr_list_once'](),
	'shuffle-once': () => m['creator.expr_list_shuffle_once'](),
	'shuffle-stopping': () => m['creator.expr_list_shuffle_stopping']()
};

export const listWord = (kind: ListKind) => LIST_WORDS[kind]();

/** chipSummary's words in the creator's language */
export const summaryWords = {
	cond: (cond: string, then: string, otherwise: string) => m['creator.expr_cond_summary']({ cond, then, otherwise: otherwise || '–' }),
	list: (kind: ListKind, items: string[]) => m['creator.expr_list_summary']({ kind: listWord(kind), items: items.join(' | ') })
};
