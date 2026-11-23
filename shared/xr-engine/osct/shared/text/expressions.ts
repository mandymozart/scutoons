/**
 * Variables in an experience's texts (content schema 2.23): the inline syntax of ink (inkle), copied from SCUTOONS' expression
 * engine (`scutoons/shared/content/src/expr/parse.ts` and `evaluate.ts`, agents/EXPRESSIONS.md there) – the parser, the
 * operator table and the evaluation rules are SCUTOONS', trimmed to what an experience has: its behaviour variables (#88), no
 * reader namespaces, no functions, no visit counts. A text is literal runs and `{…}` constructs:
 *
 *   {hasKey}                       print a variable's value (a bare camelCase name – the innermost scope that has it)
 *   {tries > 2: Try the door.}     conditional text
 *   {hasKey: You have it.|Not yet.} conditional text with an else part
 *   {name: {name}|friend}          a fallback: a variable that is not set is false, so the else part shows
 *
 * Expressions inside the braces: numbers, `"texts"`, `true` / `false`, names, `and or not && || !`, `== != < <= > >=`,
 * `+ - * / % mod`, parentheses – ink's precedence. Values work the ink way: `+` with a text joins texts, `==` with a text
 * compares texts, booleans print as true / false, whole numbers plain. A variable that is not set prints as nothing, is
 * false, equals only another unset one and makes arithmetic unset – readers never see raw code; the editor lists the
 * problem. SCUTOONS' lists (`{a|b}`, `{&…}`, `{~…}`, `{!…}`) and functions (`clock(t)`) are not part of an experience's text: they
 * parse, render nothing and come back as problems.
 *
 * Escapes: `\{`, `\}` and `\|` are the characters themselves; every other backslash stays for the Markdown that follows
 * (`entry-text.ts`). The output is Markdown source: a printed value is escaped there, so a value never becomes markup.
 * Never throws, never evaluates code, bounded by `EXPR_LIMITS` – no input can hang it.
 *
 * Content schema 5.0 (#334): a behaviour rule's condition (`when`) is one expression of the same language without braces
 * (`parseExpression`, `evaluateExpression`, `expressionVariables` below) – one language for texts and rules.
 */

/** A value an expression works with; `undefined` = no value (a variable that is not set) */
export type ExprValue = string | number | boolean | undefined;

export interface Span {
  start: number;
  end: number;
}

export type BinaryOp = "||" | "&&" | "==" | "!=" | "<" | "<=" | ">" | ">=" | "+" | "-" | "*" | "/" | "%";

export type Expr =
  | (Span & { type: "num"; value: number })
  | (Span & { type: "str"; value: string })
  | (Span & { type: "bool"; value: boolean })
  /** A name: `hasKey` – a dotted one (`map.total`) parses but no variable of the experience has one */
  | (Span & { type: "name"; path: string[] })
  /** A function call: parses, but an experience's text has no functions (`unknown-function`) */
  | (Span & { type: "call"; name: string; args: Expr[] })
  | (Span & { type: "unary"; op: "-" | "!"; arg: Expr })
  | (Span & { type: "binary"; op: BinaryOp; left: Expr; right: Expr });

/** A run of text nodes (a branch of a conditional) */
export interface Content extends Span {
  nodes: TextNode[];
}

export type TextNode =
  | (Span & { type: "text"; value: string })
  /** `{x}` */
  | (Span & { type: "print"; expr: Expr })
  /** `{cond: then|else}` */
  | (Span & { type: "cond"; cond: Expr; then: Content; else: Content | null })
  /** A construct that did not parse (or a list): renders nothing, its problem is in `problems` */
  | (Span & { type: "bad" });

/** A problem with a stable `code` (`exprMessage` words it); `args` fill the message */
export interface ExprProblem extends Span {
  code: string;
  severity: "error" | "warning";
  args?: string[];
}

export interface ParsedText {
  nodes: TextNode[];
  problems: ExprProblem[];
  /** How many `{…}` constructs (nested ones included) */
  constructs: number;
}

/** Safety limits: per expression 500 characters, depth 32, 64 nodes; per text 50 constructs and 50 000 characters */
export const EXPR_LIMITS = {
  length: 500,
  depth: 32,
  nodes: 64,
  constructs: 50,
  /** A text with `{…}` longer than this is shown as it is, nothing evaluated */
  text: 50000,
  /** Strings (values, results) are cut here */
  string: 2000,
} as const;

const L = EXPR_LIMITS;

// ── Parsing ─────────────────────────────────────────────────────────────────────────────────────

interface Tok {
  k: "num" | "str" | "id" | "op" | "p" | "eof";
  v: string;
  start: number;
  end: number;
}

interface Fail {
  fail: true;
  code: string;
  start: number;
  end: number;
  args?: string[];
}

const fail = (code: string, start: number, end: number, args?: string[]): never => {
  throw { fail: true, code, start, end, args } satisfies Fail;
};
const isFail = (e: unknown): e is Fail => !!e && (e as Fail).fail === true;

const ID = /[\p{L}_][\p{L}\p{N}_]*/uy;
const NUM = /\d+(?:\.\d+)?/y;
const WS = /[ \t\r\n]*/y;
const OPS2 = ["==", "!=", "<=", ">=", "&&", "||"];
const OPS1 = "<>+-*/%!";
const PUNCT = "(),:|}{";
const WORD_OPS = ["and", "or", "not", "mod"];
const PREC: Record<string, number> = { "||": 1, or: 1, "&&": 1, and: 1, "==": 2, "!=": 2, "<": 2, "<=": 2, ">": 2, ">=": 2, "+": 4, "-": 5, "*": 6, "/": 7, "%": 8, mod: 8 };
const NORM: Record<string, BinaryOp> = { or: "||", and: "&&", mod: "%" };

const skipWs = (src: string, i: number): number => {
  WS.lastIndex = i;
  WS.test(src);
  return WS.lastIndex;
};

const match = (re: RegExp, src: string, i: number): string | null => {
  re.lastIndex = i;
  const m = re.exec(src);
  return m ? m[0] : null;
};

/** The next token at `i` (whitespace skipped) */
function lex(src: string, i: number): Tok {
  i = skipWs(src, i);
  const c = src[i];
  if (c === undefined) return { k: "eof", v: "", start: i, end: i };
  let m = match(NUM, src, i);
  if (m) return { k: "num", v: m, start: i, end: i + m.length };
  m = match(ID, src, i);
  if (m) {
    let end = i + m.length;
    // A dotted name: the dot only joins two identifiers, no spaces
    while (src[end] === "." && (m = match(ID, src, end + 1))) end += 1 + m.length;
    const v = src.slice(i, end);
    return { k: WORD_OPS.includes(v) ? "op" : "id", v, start: i, end };
  }
  if (c === '"') {
    let v = "";
    for (let j = i + 1; j < src.length; j++) {
      const ch = src[j];
      if (ch === '"') return { k: "str", v, start: i, end: j + 1 };
      if (ch === "\\" && j + 1 < src.length) v += src[++j];
      else v += ch;
    }
    return fail("unterminated-string", i, src.length);
  }
  const two = src.slice(i, i + 2);
  if (OPS2.includes(two)) return { k: "op", v: two, start: i, end: i + 2 };
  if (OPS1.includes(c)) return { k: "op", v: c, start: i, end: i + 1 };
  if (PUNCT.includes(c)) return { k: "p", v: c, start: i, end: i + 1 };
  if (c === "=") return fail("single-equals", i, i + 1);
  if (c === "?" || c === "^") return fail("unsupported-operator", i, i + 1, [c]);
  const ch = String.fromCodePoint(src.codePointAt(i) ?? 0);
  return fail("unexpected-char", i, i + ch.length, [ch]);
}

/** One expression starting at `start` (stops before the first token that cannot continue it); throws a Fail */
function exprAt(src: string, start: number): { expr: Expr; end: number } {
  let pos = start;
  let peeked: Tok | null = null;
  let count = 0;
  const from = skipWs(src, start);
  const peek = (): Tok => (peeked ??= lex(src, pos));
  const next = (): Tok => {
    const t = peek();
    if (t.end - from > L.length) fail("too-long", from, t.end);
    pos = t.end;
    peeked = null;
    return t;
  };
  const node = <T extends Expr>(n: T): T => {
    if (++count > L.nodes) fail("too-many-nodes", n.start, n.end);
    return n;
  };
  const deep = (d: number) => {
    if (d > L.depth) fail("too-deep", peek().start, peek().end);
  };
  const text = (t: Tok) => src.slice(t.start, t.end);

  const expr = (min: number, d: number): Expr => {
    deep(d);
    let left = unary(d);
    for (;;) {
      const t = peek();
      const p = t.k === "op" ? PREC[t.v] : 0;
      if (!p || p <= min) return left;
      next();
      const right = expr(p, d + 1);
      left = node({ type: "binary", op: NORM[t.v] ?? (t.v as BinaryOp), left, right, start: left.start, end: right.end });
    }
  };
  const unary = (d: number): Expr => {
    const t = peek();
    if (t.k === "op" && (t.v === "-" || t.v === "!" || t.v === "not")) {
      deep(d + 1);
      next();
      const arg = unary(d + 1);
      return node({ type: "unary", op: t.v === "-" ? "-" : "!", arg, start: t.start, end: arg.end });
    }
    return primary(d);
  };
  const primary = (d: number): Expr => {
    const t = next();
    if (t.k === "num") return node({ type: "num", value: Number(t.v), start: t.start, end: t.end });
    if (t.k === "str") return node({ type: "str", value: t.v, start: t.start, end: t.end });
    if (t.k === "id") {
      if (t.v === "true" || t.v === "false") return node({ type: "bool", value: t.v === "true", start: t.start, end: t.end });
      if (peek().v === "(" && peek().k === "p") {
        next();
        const args: Expr[] = [];
        let close = peek();
        if (close.k === "p" && close.v === ")") next();
        else
          for (;;) {
            deep(d + 1);
            args.push(expr(0, d + 1));
            close = next();
            if (close.k === "p" && close.v === ",") continue;
            if (close.k === "p" && close.v === ")") break;
            fail(close.k === "eof" || close.k === "p" ? "missing-paren" : "unexpected", close.start, close.end, [text(close)]);
          }
        return node({ type: "call", name: t.v, args, start: t.start, end: close.end });
      }
      return node({ type: "name", path: t.v.split("."), start: t.start, end: t.end });
    }
    if (t.k === "p" && t.v === "(") {
      deep(d + 1);
      const inner = expr(0, d + 1);
      const close = next();
      if (close.k !== "p" || close.v !== ")") fail(close.k === "eof" || close.k === "p" ? "missing-paren" : "unexpected", close.start, close.end, [text(close)]);
      return inner;
    }
    if (t.k === "eof" || (t.k === "p" && t.v !== "{")) return fail("expected-value", t.start, t.end);
    return fail("unexpected", t.start, t.end, [text(t)]);
  };

  const e = expr(0, 1);
  return { expr: e, end: pos };
}

/** The characters a backslash turns into themselves here; every other backslash stays for the Markdown */
const ESCAPED = "{}|";

/** A text → its literal runs and `{…}` constructs */
export function parseText(input: string): ParsedText {
  const text = String(input ?? "");
  const n = text.length;
  const problems: ExprProblem[] = [];
  let constructs = 0;
  let i = 0;
  // At most 100 problems a text (a text of 10 000 `}` needs no 10 000 warnings)
  const problem = (code: string, start: number, end: number, args?: string[], severity: "error" | "warning" = "error") =>
    problems.length < 100 && problems.push({ code, severity, start, end, ...(args ? { args } : {}) });
  if (n > L.text && text.includes("{")) {
    problem("text-too-long", 0, n, [String(L.text)], "warning");
    return { nodes: [{ type: "text", value: text, start: 0, end: n }], problems, constructs: 0 };
  }

  /** Literal text and constructs up to the end – or, inside a construct, up to its `|` / `}` (not consumed) */
  const content = (inside: boolean, depth: number): Content => {
    const start = i;
    const nodes: TextNode[] = [];
    let buf = "";
    let from = i;
    const flush = () => {
      if (buf) nodes.push({ type: "text", value: buf, start: from, end: i });
      buf = "";
    };
    while (i < n) {
      const c = text[i];
      if (!buf) from = i;
      if (c === "\\") {
        const next = text[i + 1];
        if (next !== undefined && ESCAPED.includes(next)) buf += next;
        else buf += next === "\\" ? "\\\\" : next === undefined ? "\\" : `\\${next}`;
        i = Math.min(n, i + 2);
      } else if (c === "{") {
        flush();
        nodes.push(construct(depth));
      } else if (c === "}" && !inside) {
        problem("stray-close", i, i + 1, undefined, "warning");
        buf += c;
        i++;
      } else if ((c === "}" || c === "|") && inside) break;
      else {
        buf += c;
        i++;
      }
    }
    flush();
    return { start, end: i, nodes };
  };

  /** The `|`-separated parts from `i` up to the closing `}` – `closed` when it was there (then consumed) */
  const parts = (depth: number): { count: number; closed: boolean } => {
    let count = 1;
    content(true, depth + 1);
    while (text[i] === "|") {
      i++;
      count++;
      content(true, depth + 1);
    }
    const closed = text[i] === "}";
    if (closed) i++;
    return { count, closed };
  };

  /** Past the `}` that closes the construct whose inside starts at `from` (n when there is none) */
  const skipToClose = (from: number): number => {
    let level = 0;
    for (let j = from; j < n; j++) {
      const c = text[j];
      if (c === "\\") j++;
      else if (c === "{") level++;
      else if (c === "}" && level-- === 0) return j + 1;
    }
    return n;
  };

  const bad = (start: number): TextNode => ({ type: "bad", start, end: i });

  /** One `{…}` at `i` */
  const construct = (depth: number): TextNode => {
    const s = i;
    if (++constructs > L.constructs) {
      problem("too-many-constructs", s, n, [String(L.constructs)]);
      i = n;
      return bad(s);
    }
    if (depth >= L.depth) {
      i = skipToClose(s + 1);
      problem("too-deep", s, i, [String(L.depth)]);
      return bad(s);
    }
    i = skipWs(text, s + 1);
    // SCUTOONS' list marks (cycle, shuffle, once-only, stopping): no list in an experience's text
    if ("!&~$".includes(text[i] ?? "x")) {
      i = skipToClose(s + 1);
      problem("unsupported-list", s, i);
      return bad(s);
    }
    const p = i;
    let attempt: { expr: Expr; end: number } | null = null;
    let failure: Fail | null = null;
    try {
      attempt = exprAt(text, p);
    } catch (e) {
      if (!isFail(e)) throw e;
      failure = e;
    }
    const q = attempt ? skipWs(text, attempt.end) : -1;
    if (attempt && text[q] === ":") {
      i = q + 1;
      const then = content(true, depth + 1);
      let otherwise: Content | null = null;
      if (text[i] === "|") {
        i++;
        otherwise = content(true, depth + 1);
        if (text[i] === "|") {
          problem("conditional-too-many", i, i + 1);
          while (text[i] === "|") {
            i++;
            content(true, depth + 1);
          }
          if (text[i] === "}") i++;
          else problem("missing-close", s, n);
          return bad(s);
        }
      }
      if (text[i] !== "}") {
        problem("missing-close", s, n);
        return bad(s);
      }
      i++;
      return { type: "cond", cond: attempt.expr, then, else: otherwise, start: s, end: i };
    }
    if (attempt && text[q] === "}") {
      i = q + 1;
      return { type: "print", expr: attempt.expr, start: s, end: i };
    }
    // Two or more `|`-separated parts are a SCUTOONS sequence – no list in an experience's text
    const before = { problems: problems.length, constructs };
    i = p;
    const r = parts(depth);
    if (r.closed && r.count >= 2) {
      problems.length = before.problems;
      constructs = before.constructs;
      problem("unsupported-list", s, i);
      return bad(s);
    }
    problems.length = before.problems;
    constructs = before.constructs;
    if (!r.closed) problem("missing-close", s, n);
    else if (!text.slice(s + 1, i - 1).trim()) problem("empty-braces", s, i);
    else if (failure) problem(failure.code, failure.start, failure.end, failure.args);
    else {
      try {
        const t = lex(text, q);
        problem("unexpected", t.start, t.end, [text.slice(t.start, t.end)]);
      } catch (e) {
        if (!isFail(e)) throw e;
        problem(e.code, e.start, e.end, e.args);
      }
    }
    return bad(s);
  };

  const top = content(false, 0);
  return { nodes: top.nodes, problems, constructs };
}

// ── Evaluation ──────────────────────────────────────────────────────────────────────────────────

/** A variable's value by its name, undefined when it is not set */
export type VariableLookup = (name: string) => unknown;

export interface TextResult {
  /** The text with its constructs evaluated: Markdown source, a printed value escaped */
  text: string;
  problems: ExprProblem[];
}

interface RunFail {
  run: true;
  problem: ExprProblem;
}

const stop = (code: string, at: Span, args?: string[]): never => {
  throw { run: true, problem: { code, severity: "error", start: at.start, end: at.end, ...(args ? { args } : {}) } } satisfies RunFail;
};

/** A value as text, the ink way: whole numbers plain, fractions without float noise, booleans true / false */
export function showValue(value: ExprValue): string {
  if (value === undefined) return "";
  if (typeof value === "number") return Number.isInteger(value) ? String(value) : String(Number.parseFloat(value.toPrecision(12)));
  return String(value);
}

const clean = (value: unknown): ExprValue => {
  if (typeof value === "string") return value.length > L.string ? value.slice(0, L.string) : value;
  if (typeof value === "number") return Number.isFinite(value) ? value : undefined;
  if (typeof value === "boolean") return value;
  return undefined;
};

const truthy = (v: ExprValue): boolean => (typeof v === "string" ? v.length > 0 : !!v);

/** Is a value true the ink way (0, "", false, not set are false) */
export const isTruthy = truthy;

/** The Markdown characters a printed value must not bring in as markup */
const MARKDOWN_SPECIAL = /[\\*_[\]()<>`]/g;

/** A value as Markdown source that shows exactly the value */
export const escapeMarkdown = (text: string): string => text.replace(MARKDOWN_SPECIAL, c => `\\${c}`);

const ARITH = "+-*/%";

function runner(lookup: VariableLookup) {
  const problems: ExprProblem[] = [];

  const missing = (e: Span, what: string): undefined => {
    problems.push({ code: "missing-value", severity: "warning", start: e.start, end: e.end, args: [what] });
    return undefined;
  };

  const value = (e: Expr): ExprValue => {
    switch (e.type) {
      case "num":
      case "str":
      case "bool":
        return e.value;
      case "name": {
        let got: ExprValue;
        try {
          got = e.path.length === 1 ? clean(lookup(e.path[0])) : undefined;
        } catch {
          got = undefined;
        }
        return got === undefined ? missing(e, e.path.join(".")) : got;
      }
      case "call":
        return stop("unknown-function", e, [e.name]);
      case "unary": {
        const x = value(e.arg);
        if (e.op === "!") return !truthy(x);
        if (x === undefined) return undefined;
        return typeof x === "string" ? stop("not-a-number", e, ["-"]) : -Number(x);
      }
      case "binary":
        return binary(e);
    }
  };

  const binary = (e: Expr & { type: "binary" }): ExprValue => {
    const op = e.op;
    if (op === "&&") return truthy(value(e.left)) && truthy(value(e.right));
    if (op === "||") return truthy(value(e.left)) || truthy(value(e.right));
    const a = value(e.left);
    const b = value(e.right);
    const text = typeof a === "string" || typeof b === "string";
    const arith = ARITH.includes(op);
    if (op === "==" || op === "!=") {
      const same = a === undefined || b === undefined ? a === b : text ? showValue(a) === showValue(b) : Number(a) === Number(b);
      return op === "==" ? same : !same;
    }
    if (op === "+" && text) return (showValue(a) + showValue(b)).slice(0, L.string);
    if (text) return stop(arith ? "not-a-number" : "compare-text", e, [op]);
    if (a === undefined || b === undefined) return arith ? undefined : false;
    const x = Number(a);
    const y = Number(b);
    if (op === "<") return x < y;
    if (op === "<=") return x <= y;
    if (op === ">") return x > y;
    if (op === ">=") return x >= y;
    if ((op === "/" || op === "%") && y === 0) return stop("division-by-zero", e);
    const r = op === "+" ? x + y : op === "-" ? x - y : op === "*" ? x * y : op === "/" ? x / y : x % y;
    return Number.isFinite(r) ? r : stop("not-a-number", e, [op]);
  };

  /** An expression's value – ok false plus a problem when it cannot be evaluated */
  const guarded = (e: Expr): { ok: boolean; value: ExprValue } => {
    try {
      return { ok: true, value: value(e) };
    } catch (x) {
      if (!(x as RunFail)?.run) throw x;
      problems.push((x as RunFail).problem);
      return { ok: false, value: undefined };
    }
  };

  const content = (c: Content | null): string => (c ? c.nodes.map(node).join("") : "");

  const node = (n: TextNode): string => {
    if (n.type === "text") return n.value;
    if (n.type === "print") {
      const r = guarded(n.expr);
      return r.ok ? escapeMarkdown(showValue(r.value)) : "";
    }
    if (n.type === "cond") {
      const r = guarded(n.cond);
      return !r.ok ? "" : truthy(r.value) ? content(n.then) : content(n.else);
    }
    return "";
  };

  return { problems, node, guarded };
}

/** ink's output clean-up: runs of spaces collapse to one, no spaces at the start / end of a line */
const tidy = (text: string): string =>
  text
    .split("\n")
    .map(line => line.replace(/[ \t]+/g, " ").replace(/^ | $/g, ""))
    .join("\n");

/** A text with its `{…}` evaluated against the variables; a text without `{ } \` comes back as it is */
export function evaluateText(text: string, lookup: VariableLookup): TextResult {
  const source = String(text ?? "");
  if (!/[{}\\]/.test(source)) return { text: source, problems: [] };
  const parsed = parseText(source);
  const run = runner(lookup);
  const out = parsed.nodes.map(run.node).join("");
  return { text: parsed.constructs ? tidy(out) : out, problems: [...parsed.problems, ...run.problems] };
}

/** Every variable name an expression reads, into `names` in the order of the expression */
const exprNames = (e: Expr, names: Set<string>): void => {
  if (e.type === "name") names.add(e.path.join("."));
  else if (e.type === "call") e.args.forEach(arg => exprNames(arg, names));
  else if (e.type === "unary") exprNames(e.arg, names);
  else if (e.type === "binary") {
    exprNames(e.left, names);
    exprNames(e.right, names);
  }
};

/** The variables a text reads, each once in the order of the text (the editor shows their values) */
export function textVariables(text: string): string[] {
  const names = new Set<string>();
  const nodes = (list: TextNode[]): void =>
    list.forEach(n => {
      if (n.type === "print") exprNames(n.expr, names);
      else if (n.type === "cond") {
        exprNames(n.cond, names);
        nodes(n.then.nodes);
        if (n.else) nodes(n.else.nodes);
      }
    });
  if (String(text ?? "").includes("{")) nodes(parseText(text).nodes);
  return [...names];
}

// ── Standalone expressions (content schema 5.0, #334): a rule's `when` ───────────────────────────
//
// The same language without braces – `hasKey && tries > 2`, `mood == "happy" || !door` – parsed by the same parser and
// run by the same runner as a text's `{…}`. A rule's condition passes when its value is true the ink way (`isTruthy`).
// Functions and dotted names parse but are problems: an experience's variables are plain camelCase names.

export interface ParsedExpression {
  /** null when it does not parse (its problem is in `problems`) */
  expr: Expr | null;
  /** Errors: it does not parse, or it uses what a rule cannot (a function, a dotted name) */
  problems: ExprProblem[];
}

export interface ExpressionResult {
  /** The expression's value; undefined when it could not be evaluated (or a variable it is made of is not set) */
  value: ExprValue;
  /** false when it does not parse or cannot be evaluated (a text compared with `<`, division by zero, a function) */
  ok: boolean;
  /** The parse problems and the runner's (a variable that is not set is a warning) */
  problems: ExprProblem[];
}

/** What a parsed expression uses that a rule cannot: functions and dotted names */
const ruleProblems = (e: Expr, problems: ExprProblem[]): void => {
  if (e.type === "call") problems.push({ code: "unknown-function", severity: "error", start: e.start, end: e.end, args: [e.name] });
  else if (e.type === "name" && e.path.length > 1) problems.push({ code: "dotted-name", severity: "error", start: e.start, end: e.end, args: [e.path.join(".")] });
  if (e.type === "call") e.args.forEach(arg => ruleProblems(arg, problems));
  else if (e.type === "unary") ruleProblems(e.arg, problems);
  else if (e.type === "binary") {
    ruleProblems(e.left, problems);
    ruleProblems(e.right, problems);
  }
};

/** One expression (a rule's `when`): the whole source must be one expression – never throws */
export function parseExpression(source: string): ParsedExpression {
  const src = String(source ?? "");
  if (!src.trim()) return { expr: null, problems: [{ code: "empty-expression", severity: "error", start: 0, end: src.length }] };
  try {
    const { expr, end } = exprAt(src, 0);
    const rest = skipWs(src, end);
    if (rest < src.length) {
      const t = lex(src, rest);
      return { expr: null, problems: [{ code: "unexpected", severity: "error", start: t.start, end: t.end, args: [src.slice(t.start, t.end)] }] };
    }
    const problems: ExprProblem[] = [];
    ruleProblems(expr, problems);
    return { expr, problems };
  } catch (e) {
    if (!isFail(e)) throw e;
    return { expr: null, problems: [{ code: e.code, severity: "error", start: e.start, end: e.end, ...(e.args ? { args: e.args } : {}) }] };
  }
}

/** An expression's value against the variables (a source, or one `parseExpression` parsed) – never throws */
export function evaluateExpression(source: string | Expr, lookup: VariableLookup): ExpressionResult {
  const parsed: ParsedExpression = typeof source === "string" ? parseExpression(source) : { expr: source, problems: [] };
  if (!parsed.expr || parsed.problems.some(problem => problem.severity === "error")) return { value: undefined, ok: false, problems: parsed.problems };
  const run = runner(lookup);
  const result = run.guarded(parsed.expr);
  return { value: result.value, ok: result.ok, problems: [...parsed.problems, ...run.problems] };
}

/** The variables an expression reads, each once in the order of the expression ([] when it does not parse) */
export function expressionVariables(source: string): string[] {
  const { expr } = parseExpression(source);
  const names = new Set<string>();
  if (expr) exprNames(expr, names);
  return [...names];
}

/** A value as an expression's literal: `"a \"quoted\" text"`, `3`, `2.5`, `true` (a migration or an editor writes it) */
export function expressionLiteral(value: string | number | boolean): string {
  if (typeof value === "string") return `"${value.replace(/["\\]/g, c => `\\${c}`)}"`;
  if (typeof value === "number") return showValue(value);
  return String(value);
}

const MESSAGES: Record<string, string> = {
  "unterminated-string": "a text in quotes is not closed",
  "single-equals": "= is no comparison – write ==",
  "unsupported-operator": "{0} is no operator here",
  "unexpected-char": "{0} is not allowed in braces",
  unexpected: "{0} does not belong here",
  "too-long": "the expression is too long",
  "too-many-nodes": "the expression is too long",
  "too-deep": "nested too deeply",
  "missing-paren": "a ( is not closed",
  "expected-value": "a value is missing",
  "stray-close": "a } without its { – write \\} for the character",
  "missing-close": "a { is not closed – write \\{ for the character",
  "empty-braces": "empty braces",
  "too-many-constructs": "more than {0} variables in one text",
  "text-too-long": "the text is longer than {0} characters: its variables are not filled in",
  "conditional-too-many": "a condition has one else part: {condition: then|else}",
  "unsupported-list": "lists ({a|b}) are no part of an experience's text",
  "unknown-function": "{0}() – an experience's texts and rules have no functions",
  "dotted-name": "{0} – a variable's name has no dot",
  "empty-expression": "the condition is empty",
  "missing-value": "{0} is not set – it shows as nothing",
  "not-a-number": "{0} needs numbers",
  "compare-text": "{0} compares numbers, not texts",
  "division-by-zero": "division by zero",
};

/** A problem in words, for the editor */
export function exprMessage(problem: Pick<ExprProblem, "code" | "args">): string {
  const template = MESSAGES[problem.code] ?? problem.code;
  return template.replace(/\{(\d)\}/g, (_, i: string) => problem.args?.[Number(i)] ?? "");
}
