// Expressions in texts (agents/EXPRESSIONS.md): the syntax tree of ink's inline syntax. Every node carries its source
// span – `start` / `end` are UTF-16 offsets into the text (what JS strings and the creator's editor count), end exclusive.

/** a value an expression works with; `undefined` = no value (a name the reader has no answer for) */
export type ExprValue = string | number | boolean | undefined;

export interface Span {
	start: number;
	end: number;
}

export type BinaryOp = '||' | '&&' | '==' | '!=' | '<' | '<=' | '>' | '>=' | '+' | '-' | '*' | '/' | '%';

export type Expr =
	| (Span & { type: 'num'; value: number })
	| (Span & { type: 'str'; value: string })
	| (Span & { type: 'bool'; value: boolean })
	/** a name: `coins` (story variable) or `map.total` (a namespace's member) – never a property read on a value */
	| (Span & { type: 'name'; path: string[] })
	/** a registered function: `clock(t)`, `inventory.has("x")` (dotted name joined with `.`) */
	| (Span & { type: 'call'; name: string; args: Expr[] })
	| (Span & { type: 'unary'; op: '-' | '!'; arg: Expr })
	| (Span & { type: 'binary'; op: BinaryOp; left: Expr; right: Expr });

/** ink's sequence kinds: `{a|b}` / `{$a|b}` sequence (stops on the last), `{&…}` cycle, `{~…}` shuffle, `{!…}` once-only,
 *  `{~!…}` shuffle once, `{~$…}` shuffle stopping */
export type ListKind = 'sequence' | 'cycle' | 'shuffle' | 'once' | 'shuffle-once' | 'shuffle-stopping';

/** a run of text nodes (a branch of a conditional, an element of a list) */
export interface Content extends Span {
	nodes: TextNode[];
}

export type TextNode =
	| (Span & { type: 'text'; value: string })
	/** `{x}` */
	| (Span & { type: 'print'; expr: Expr })
	/** `{cond: then|else}` */
	| (Span & { type: 'cond'; cond: Expr; then: Content; else: Content | null })
	/** `{a|b}`, `{&a|b}` … – `index` = the list's number in the text (document order, from 0): its visit-count key */
	| (Span & { type: 'list'; kind: ListKind; index: number; items: Content[] })
	/** a construct that didn't parse: renders nothing (its problem is in `problems`) */
	| (Span & { type: 'bad' });

/** a problem with a stable `code` (messages: exprMessage); `args` fill the message, `suggestion` = "did you mean" */
export interface ExprProblem extends Span {
	code: string;
	severity: 'error' | 'warning';
	args?: string[];
	suggestion?: string;
}

export interface ParsedText {
	nodes: TextNode[];
	problems: ExprProblem[];
	/** how many `{…}` constructs (nested ones included) */
	constructs: number;
	/** how many lists (sequences / cycles / shuffles / once-only) – their `index` runs 0 … lists-1 */
	lists: number;
}

/** safety limits (EXPRESSIONS.md §1): per expression 500 characters, depth 32, 64 nodes; per text 50 constructs */
export const EXPR_LIMITS = {
	length: 500,
	depth: 32,
	nodes: 64,
	constructs: 50,
	/** a text with `{…}` longer than this isn't evaluated */
	text: 10000,
	/** strings (values, results) are cut here */
	string: 2000,
} as const;
