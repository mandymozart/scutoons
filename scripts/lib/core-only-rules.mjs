// core-only-rules.mjs (#363, guard v2): the shapes deck logic takes in the browser's TypeScript, one file's text at a time,
// for scripts/check-core-only.mjs. Two layers:
// - v1 (#357): regex patterns over each line's code (PATTERNS – their names are the old baseline keys, unchanged);
// - v2 (#363): the TypeScript AST (the `typescript` the apps already build with) – the same shapes in any form, and the
//   ones a regex cannot see: catalog kind names from kinds.json in literals and branches, clamps as helpers / ternaries /
//   variable bounds, tempo arithmetic by identifier, MIDI status as decimal or a named const, PRNG constants, a renamed
//   lead rule, `{min, max, value}` tables, the being's mood names, numbers that clone a deck Rust constant, and comments
//   that name a Rust twin without a ticket (BRIDGE-AUDIT.md §5). A v2 hit on a line its v1 family already matched is
//   dropped, so the old baseline counts stay what they were.
import ts from 'typescript';

/** v1: the shapes deck logic takes in TypeScript, as line regexes – each name is a baseline key */
export const PATTERNS = {
	'scene-literal': { re: /\bkind:\s*['"][a-z_]+['"]/g, what: 'a scene or patch written in TS (a node list) – scenes are TOML the core loads' },
	// (a frame rate's `/ 60` is no tempo; a minute's `/ 60_000` next to a beat is)
	'tempo-math': { re: /\bPPQN\b|\/\s*60_?000\b|\b60(?:_?000)?\s*\/\s*[\w.]*bpm|\bbpm\s*[*/]|[*/]\s*[\w.]*\bbpm\b|Math\.floor\(\s*beat/gi, what: 'tempo or beat arithmetic – the clock is deck-core\'s (tempo::Controller)' },
	// (a byte mask `& 0xff` is no MIDI)
	'midi-bytes': { re: /(?<!&\s*)\b0x(?:f[0-9a-e]|[89ab][0-9a-f])\b/gi, what: 'MIDI status bytes – MIDI is parsed and made in deck-core (tempo::midi_in, clock_out)' },
	'core-text': { re: /NO SHOW CLOCK|SHOW CLOCK SYNCING|NO (?:MIDI )?CLOCK|\bHOLDS\b|['"`\s]LEADS\b|SHOW \$\{/g, what: 'a status text the core produces (Controller::status) – show the core\'s line' },
	clamp: { re: /Math\.min\(\s*-?[\d.]+\s*,\s*Math\.max\(|Math\.max\(\s*-?[\d.]+\s*,\s*Math\.min\(/g, what: 'a clamp with fixed bounds – ranges and defaults are the catalog\'s (deck/catalog, kinds.json)' },
	'lead-rule': { re: /\bAdaptiveLead\b/g, what: 'the audio lead rule – deck-core\'s (lead.rs)' },
	random: { re: /Math\.random\(/g, what: 'randomness – deck behaviour must be the core\'s (seeded, the same on every target)' },
	'kind-branch': { re: /\bkind\s*[!=]==?\s*['"]/g, what: 'branching on a catalog kind name – kinds behave in the core' }
};

/** v2: the AST rules – name → [the v1 family it extends (a line that family matched is not counted twice), what it means] */
export const RULES = {
	'catalog-literal': ['scene-literal', 'a node of the kind catalog written in TS (an object, a TOML / JSON string, a node() helper, a kind list or a table keyed by kinds) – scenes come from deck/ files the core loads'],
	'catalog-branch': ['kind-branch', 'a decision on a catalog kind (===, switch, includes, a map lookup) – kinds behave in the core'],
	'clamp-shape': ['clamp', 'a clamp (a helper, a ternary, min / max in either order or with variable bounds) – ranges are the catalog\'s, the core clamps'],
	'tempo-shape': ['tempo-math', 'arithmetic on a tempo, beat or bpm value – the clock is deck-core\'s'],
	'midi-shape': ['midi-bytes', 'a MIDI status as a mask, a decimal or a named constant – MIDI is parsed in deck-core'],
	'text-shape': ['core-text', 'a status / HUD text the core (or main.rs, #374) builds – built from parts, lower case or a template; show the core\'s line'],
	'random-shape': ['random', 'randomness by another name (crypto, an LCG / xorshift constant) – the core\'s seeded generators'],
	'lead-shape': ['lead-rule', 'a lead / buffer size rule in audio frames – the lead is the core\'s rule (#365)'],
	threshold: [null, 'a ladder of thresholds on one value (`x > 16 ? \'bad\' : x > 10 ? \'warn\' : …`) – the judgement of a number is the core\'s (#364)'],
	'spec-table': [null, 'a parameter table ({min, max, default / value / step}) – ranges, defaults and labels are the catalog\'s'],
	'mood-literal': [null, 'one of the being\'s mood names (deck-space being.rs MOODS) – the mood ladder is the core\'s (#367)'],
	'twin-const': [null, 'a number that clones a deck Rust constant or array – a twin; take it from the core'],
	'twin-comment': [null, 'a comment that names a Rust twin ("twin of", "as the native", ".rs", "trimmed" …) without a ticket – add the move ticket']
};

/** what every rule name means (v1 and v2) */
export const WHAT = Object.fromEntries([...Object.entries(PATTERNS).map(([k, v]) => [k, v.what]), ...Object.entries(RULES).map(([k, v]) => [k, v[1]])]);
export const RULE_NAMES = Object.keys(WHAT);

const ESCAPE = /core-only-ok:\s*(.*)$/;
const TWIN_PHRASE = /twin of|\bas the native\b|\bthe native(?:'s)? (?:rule|copy|one)|\bsame rules?\b|\.rs\b|\btrimmed\b|kept in TS|for now in the page|\bmirrors? (?:the )?(?:core|native|rust|deck)|copy of the (?:core|native|rust)/i;
// the core's clock texts in any case (Controller::status), and the HUD's words in upper case (main.rs bar_items / hud_lines)
const CLOCK_TEXT = /\bno show clock\b|\bshow clock (?:yet|syncing)\b|\bsyncing\b|\bno (?:midi )?clock\b|\bholds\b|\bleads\b|\bshow \u0000|\brtt \u0000/i;
const HUD_TEXT = /\b(?:FPS|CPU|DSP|GLITCH(?:ES)?|UNDERRUNS?|LIM|BPM \u0000|BAR \u0000|BEAT \u0000|AUDIO (?:OFF|GLITCH)|RELAY|Q \u0000)\b/;
const TEMPO_NAME = /bpm|tempo|beat|ppqn|perbar|per_bar/i;
const MIDI_NAME = /status|cmd|command|midi|byte|data\b|\btype$/i;
const MIDI_CONST = /^(?:NOTE_?ON|NOTE_?OFF|CC|CONTROL(?:_?CHANGE)?|PROGRAM(?:_?CHANGE)?|PITCH(?:_?BEND)?|AFTERTOUCH|CLOCK|TIMING(?:_?CLOCK)?|TICK|START|STOP|CONTINUE|SONG_?POS(?:ITION)?|SYSEX|SPP|MIDI_\w+)$/i;
const LEAD_NAME = /lead|latency|ahead|cushion|bufferframes|buffered|queued?frames|target_?frames/i;
// LCG / hash multipliers and increments (Numerical Recipes, glibc, MSVC, MINSTD, mulberry32, the golden ratio, Java's)
const PRNG = new Set([1664525, 1013904223, 22695477, 1103515245, 214013, 2531011, 48271, 16807, 0x6d2b79f5, 0x9e3779b9, 2654435761, 0x5deece66d, 0x85ebca6b, 0xc2b2ae35]);
/** numbers too common to call a twin (frame counts, rates, units) */
const COMMON = new Set([0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 16, 24, 30, 32, 60, 64, 100, 127, 128, 255, 256, 360, 512, 1000, 1024, 2048, 4096, 3600, 44100, 48000, 60000, 65535, 65536, 96000, 1e6, 1e9]);

/** a numeric literal's value (`20_000`, `0x90`, `1e3`) */
export function num(text) {
	const t = String(text).replace(/_/g, '');
	return /^0[xob]/i.test(t) ? Number(t) : parseFloat(t);
}

/** whether `v` is distinctive enough that the same number in TS and in a Rust constant is a twin, not chance */
export function distinctive(v) {
	if (!Number.isFinite(v) || COMMON.has(Math.abs(v))) return false;
	// an integer of three digits or more that is no power of two and no round timer below 10 s (20_000, 3072, 147 – not
	// 512, 4096, 250, 500)
	const a = Math.abs(v);
	if (Number.isInteger(v)) return a >= 100 && (a & (a - 1)) !== 0 && !(a < 10_000 && a % 50 === 0);
	// a fraction with two or more significant digits that is no quarter / eighth (0.88, 0.15, 0.175 – not 0.5, 0.25, 0.1)
	if (Number.isInteger(v * 8)) return false;
	return !/^-?0?\.0*[1-9]$/.test(String(v));
}

/** whether a list of numbers is a table worth comparing (4+ values or a distinctive one; no 0, 1, 2, 3 … range) */
export function tableLike(values) {
	const u = [...new Set(values)].sort((a, b) => a - b);
	if (u.length < 3) return false;
	if (u.every((v, i) => i === 0 || v - u[i - 1] === 1)) return false;
	return u.length >= 4 || u.some(distinctive);
}

/** One line's code without its comments, and its comment text (a `//` after a `:` or a quote is kept: URLs, strings). */
function split(line, state) {
	let s = line;
	let comment = '';
	if (state.block) {
		const end = s.indexOf('*/');
		if (end < 0) return { code: '', comment: s };
		comment += s.slice(0, end);
		s = s.slice(end + 2);
		state.block = false;
	}
	s = s.replace(/\/\*(.*?)\*\//g, (_, c) => ((comment += ` ${c}`), '')).replace(/<!--(.*?)-->/g, (_, c) => ((comment += ` ${c}`), ''));
	const open = s.indexOf('/*');
	if (open >= 0) {
		state.block = true;
		comment += s.slice(open + 2);
		s = s.slice(0, open);
	}
	if (/^\s*\*/.test(s)) return { code: '', comment: comment + s };
	const m = /(^|[^:'"`\\\w])\/\/(.*)$/.exec(s);
	if (m) {
		comment += m[2];
		s = s.slice(0, m.index + m[1].length);
	}
	return { code: s, comment };
}

/** the script blocks of a Svelte file (or the whole TS file): [text, first line (0-based)] */
function scripts(text, svelte) {
	if (!svelte) return [[text, 0]];
	const out = [];
	for (const m of text.matchAll(/<script\b[^>]*>([\s\S]*?)<\/script>/g)) {
		const start = m.index + m[0].indexOf('>') + 1;
		out.push([m[1], text.slice(0, start).split('\n').length - 1]);
	}
	return out;
}

const nameOf = (e) => {
	if (!e) return '';
	if (ts.isIdentifier(e) || ts.isPrivateIdentifier(e)) return e.text;
	if (ts.isPropertyAccessExpression(e)) return e.name.text;
	if (ts.isElementAccessExpression(e)) return ts.isStringLiteralLike(e.argumentExpression) ? e.argumentExpression.text : nameOf(e.expression);
	if (ts.isParenthesizedExpression(e) || ts.isNonNullExpression(e) || ts.isAsExpression(e)) return nameOf(e.expression);
	if (ts.isCallExpression(e)) return nameOf(e.expression);
	return '';
};
const kindish = (e) => /kind$/i.test(nameOf(e)) || nameOf(e) === 'type';
const str = (e) => (e && (ts.isStringLiteral(e) || ts.isNoSubstitutionTemplateLiteral(e)) ? e.text : null);
const value = (e) => {
	if (!e) return null;
	if (ts.isNumericLiteral(e)) return num(e.text);
	if (ts.isPrefixUnaryExpression(e) && e.operator === ts.SyntaxKind.MinusToken && ts.isNumericLiteral(e.operand)) return -num(e.operand.text);
	if (ts.isParenthesizedExpression(e)) return value(e.expression);
	return null;
};
const propName = (p) => (p.name && (ts.isIdentifier(p.name) || ts.isStringLiteralLike(p.name) || ts.isNumericLiteral(p.name)) ? p.name.text : '');
const COMPARE = new Set([ts.SyntaxKind.EqualsEqualsEqualsToken, ts.SyntaxKind.EqualsEqualsToken, ts.SyntaxKind.ExclamationEqualsEqualsToken, ts.SyntaxKind.ExclamationEqualsToken]);
const RELATION = new Set([ts.SyntaxKind.LessThanToken, ts.SyntaxKind.GreaterThanToken, ts.SyntaxKind.LessThanEqualsToken, ts.SyntaxKind.GreaterThanEqualsToken]);
const ARITH = new Set([ts.SyntaxKind.AsteriskToken, ts.SyntaxKind.SlashToken, ts.SyntaxKind.PercentToken, ts.SyntaxKind.AsteriskEqualsToken, ts.SyntaxKind.SlashEqualsToken]);
const ASSIGN = new Set([ts.SyntaxKind.EqualsToken, ts.SyntaxKind.PlusEqualsToken, ts.SyntaxKind.MinusEqualsToken, ts.SyntaxKind.AsteriskEqualsToken]);

/** the strings an expression can evaluate to, folded from literals (`+`, templates – a substitution is \u0000 –, `.join`, ?:, case) */
function fold(e, depth = 0) {
	if (!e || depth > 6) return null;
	const s = str(e);
	if (s !== null) return [s];
	if (ts.isTemplateExpression(e)) return [e.head.text + e.templateSpans.map((sp) => `\u0000${sp.literal.text}`).join('')];
	if (ts.isParenthesizedExpression(e)) return fold(e.expression, depth + 1);
	if (ts.isConditionalExpression(e)) {
		const a = fold(e.whenTrue, depth + 1);
		const b = fold(e.whenFalse, depth + 1);
		return a || b ? [...(a ?? ['\u0000']), ...(b ?? ['\u0000'])] : null;
	}
	if (ts.isBinaryExpression(e) && e.operatorToken.kind === ts.SyntaxKind.PlusToken) {
		const a = fold(e.left, depth + 1);
		const b = fold(e.right, depth + 1);
		if (!a && !b) return null;
		return (a ?? ['\u0000']).flatMap((x) => (b ?? ['\u0000']).map((y) => x + y)).slice(0, 16);
	}
	if (ts.isCallExpression(e) && ts.isPropertyAccessExpression(e.expression)) {
		const m = e.expression.name.text;
		if (m === 'join' && ts.isArrayLiteralExpression(e.expression.expression)) {
			const parts = e.expression.expression.elements.map((x) => fold(x, depth + 1)?.[0] ?? '\u0000');
			const sep = e.arguments.length ? (str(e.arguments[0]) ?? ' ') : ',';
			return parts.some((p) => p !== '\u0000') ? [parts.join(sep)] : null;
		}
		if (/^to(?:Upper|Lower)Case$/.test(m)) return fold(e.expression.expression, depth + 1);
	}
	return null;
}

/** the deck's vocabulary the v2 rules match against: catalog kinds, mood names, Rust constants and arrays */
export function emptyContext() {
	return { kinds: new Set(), moods: new Set(), consts: new Map(), arrays: [] };
}

/** the AST rules over one script block → hits [{rule, line (0-based in the block)}] */
function astHits(text, ctx) {
	const sf = ts.createSourceFile('x.ts', text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
	const hits = [];
	const at = (n) => sf.getLineAndCharacterOfPosition(n.getStart(sf)).line;
	const hit = (rule, n) => hits.push({ rule, line: at(n) });
	const folded = new Set();
	const visit = (n) => {
		// catalog literals: { kind: 'fm' } · `kind = "fm"` / "kind": "fm" / [[node]] in a string · node('fm', …) · ['fm', 'osc'] ·
		// { fm: …, osc: … }
		if (ts.isPropertyAssignment(n) && propName(n) === 'kind' && ctx.kinds.has(str(n.initializer))) hit('catalog-literal', n);
		if (ts.isStringLiteralLike(n) || ts.isTemplateExpression(n)) {
			const t = ts.isTemplateExpression(n) ? n.head.text + n.templateSpans.map((s) => ` ${s.literal.text}`).join('') : n.text;
			const kinds = [...t.matchAll(/\bkind\s*=\s*["']([a-z_0-9]+)["']|"kind"\s*:\s*"([a-z_0-9]+)"/g)].map((m) => m[1] ?? m[2]);
			if (kinds.some((k) => ctx.kinds.has(k)) || /\[\[node\]\]/.test(t)) hit('catalog-literal', n);
		}
		if (ts.isCallExpression(n) && /node|kind/i.test(nameOf(n.expression)) && ctx.kinds.has(str(n.arguments[0]))) hit('catalog-literal', n);
		if (ts.isArrayLiteralExpression(n)) {
			const ss = n.elements.map(str);
			if (ss.length >= 2 && ss.every((s) => s !== null && ctx.kinds.has(s))) hit('catalog-literal', n);
			const vs = n.elements.map(value);
			if (vs.length >= 3 && vs.every((v) => v !== null) && tableLike(vs)) {
				const key = [...new Set(vs)].sort((a, b) => a - b).join(',');
				if (ctx.arrays.some((a) => a.key === key)) hit('twin-const', n);
			}
		}
		if (ts.isObjectLiteralExpression(n)) {
			const names = n.properties.map(propName);
			if (names.filter((p) => ctx.kinds.has(p)).length >= 2 && !(ts.isElementAccessExpression(n.parent) && kindish(n.parent.argumentExpression))) hit('catalog-literal', n);
			const nums = new Set(n.properties.filter((p) => ts.isPropertyAssignment(p) && value(p.initializer) !== null).map(propName));
			if (nums.has('min') && nums.has('max') && ['default', 'value', 'def', 'step', 'init', 'neutral'].some((k) => nums.has(k))) hit('spec-table', n);
		}
		// catalog branches: x.kind === 'fm' · switch (x.kind) { case 'fm' } · [...].includes(x.kind) · { fm: … }[x.kind]
		if (ts.isBinaryExpression(n) && COMPARE.has(n.operatorToken.kind)) {
			const [a, b] = [n.left, n.right];
			if ((ctx.kinds.has(str(a)) && kindish(b)) || (ctx.kinds.has(str(b)) && kindish(a))) hit('catalog-branch', n);
		}
		if (ts.isSwitchStatement(n) && kindish(n.expression)) {
			for (const c of n.caseBlock.clauses) if (ts.isCaseClause(c) && ctx.kinds.has(str(c.expression))) hit('catalog-branch', c);
		}
		if (ts.isCallExpression(n) && ts.isPropertyAccessExpression(n.expression) && /^(includes|has|indexOf)$/.test(n.expression.name.text) && kindish(n.arguments[0])) {
			let r = n.expression.expression;
			if (ts.isNewExpression(r) && r.arguments?.length) r = r.arguments[0];
			if (ts.isArrayLiteralExpression(r) && r.elements.some((x) => ctx.kinds.has(str(x)))) hit('catalog-branch', n);
		}
		if (ts.isElementAccessExpression(n) && kindish(n.argumentExpression)) {
			const o = ts.isParenthesizedExpression(n.expression) ? n.expression.expression : n.expression;
			if (ts.isObjectLiteralExpression(o) && o.properties.some((p) => ctx.kinds.has(propName(p)))) hit('catalog-branch', n);
		}
		// clamps: Math.min(…Math.max(…)…) in either order, any bounds · clamp(…) · a ? : chain on one value
		if (ts.isCallExpression(n)) {
			const callee = n.expression;
			const mm = (e) => ts.isCallExpression(e) && ts.isPropertyAccessExpression(e.expression) && nameOf(e.expression.expression) === 'Math' && /^(min|max)$/.test(e.expression.name.text) ? e.expression.name.text : null;
			const outer = mm(n);
			if (outer && n.arguments.some((a) => { const inner = mm(ts.isParenthesizedExpression(a) ? a.expression : a); return inner && inner !== outer; })) hit('clamp-shape', n);
			if (!outer && /^clamp|clamp(?:01)?$/i.test(nameOf(callee)) && n.arguments.length) hit('clamp-shape', n);
		}
		if (ts.isConditionalExpression(n) && !ts.isConditionalExpression(n.parent)) {
			const inner = [n.whenTrue, n.whenFalse].find((x) => ts.isConditionalExpression(x));
			const ops = (c) => ts.isBinaryExpression(c) && RELATION.has(c.operatorToken.kind) ? [c.left.getText(sf), c.right.getText(sf)] : [];
			if (inner) {
				const a = ops(n.condition);
				const b = ops(inner.condition);
				const shared = a.find((x) => b.includes(x));
				// the value itself among the results: a clamp; else a ladder of thresholds on one value (a judgement)
				const leaves = [n.whenTrue, n.whenFalse, inner.whenTrue, inner.whenFalse].map((x) => x.getText(sf));
				if (shared !== undefined) hit(leaves.includes(shared) ? 'clamp-shape' : 'threshold', n);
			}
		}
		// tempo arithmetic by name; MIDI status shapes
		if (ts.isBinaryExpression(n) && ARITH.has(n.operatorToken.kind) && (TEMPO_NAME.test(nameOf(n.left)) || TEMPO_NAME.test(nameOf(n.right)))) hit('tempo-shape', n);
		if (ts.isBinaryExpression(n)) {
			const k = n.operatorToken.kind;
			if (k === ts.SyntaxKind.AmpersandToken && (value(n.right) === 0xf0 || value(n.left) === 0xf0)) hit('midi-shape', n);
			if (k === ts.SyntaxKind.GreaterThanGreaterThanToken && value(n.right) === 4 && MIDI_NAME.test(nameOf(n.left))) hit('midi-shape', n);
			if (COMPARE.has(k)) {
				for (const [x, y] of [[n.left, n.right], [n.right, n.left]]) {
					const v = value(x);
					const yy = ts.isParenthesizedExpression(y) ? y.expression : y;
					const masked = ts.isBinaryExpression(yy) && yy.operatorToken.kind === ts.SyntaxKind.AmpersandToken;
					if (v !== null && v >= 128 && v <= 255 && Number.isInteger(v) && (masked || MIDI_NAME.test(nameOf(y)))) hit('midi-shape', n);
				}
			}
		}
		if (ts.isCaseClause(n) && ts.isSwitchStatement(n.parent.parent) && MIDI_NAME.test(nameOf(n.parent.parent.expression))) {
			const v = value(n.expression);
			if (v !== null && v >= 128 && v <= 255 && !/^0x/i.test(n.expression.getText(sf))) hit('midi-shape', n);
		}
		if (ts.isVariableDeclaration(n) && ts.isIdentifier(n.name) && MIDI_CONST.test(n.name.text)) {
			const v = value(n.initializer);
			if (v !== null && v >= 128 && v <= 255 && !/^0x/i.test(n.initializer.getText(sf))) hit('midi-shape', n);
		}
		// randomness by another name
		if (ts.isPropertyAccessExpression(n) && n.name.text === 'getRandomValues') hit('random-shape', n);
		if (ts.isNumericLiteral(n) && PRNG.has(num(n.text))) hit('random-shape', n);
		if (ts.isBinaryExpression(n) && n.operatorToken.kind === ts.SyntaxKind.CaretEqualsToken) {
			const r = ts.isParenthesizedExpression(n.right) ? n.right.expression : n.right;
			if (ts.isBinaryExpression(r) && /^(<<|>>|>>>)$/.test(r.operatorToken.getText(sf)) && r.left.getText(sf) === n.left.getText(sf)) hit('random-shape', n);
		}
		// a lead rule: a lead / latency / buffer target set from a frame count (a multiple of the 128-frame quantum), or any
		// value that grows from itself within min / max bounds in frame counts (`x = Math.min(x + 512, 6144)` – adaptive)
		const target = ts.isBinaryExpression(n) && ASSIGN.has(n.operatorToken.kind) ? [nameOf(n.left), n.right, n.left.getText(sf)]
			: (ts.isVariableDeclaration(n) || ts.isPropertyDeclaration(n)) && n.initializer ? [ts.isIdentifier(n.name) ? n.name.text : propName(n), n.initializer, null] : null;
		if (target) {
			let frames = false;
			let bounded = false;
			let self = false;
			const look = (e) => {
				const v = value(e);
				if (v !== null && v >= 256 && v % 128 === 0) frames = true;
				else {
					if (ts.isCallExpression(e) && ts.isPropertyAccessExpression(e.expression) && nameOf(e.expression.expression) === 'Math' && /^(min|max)$/.test(e.expression.name.text)) bounded = true;
					if (target[2] && e.getText(sf) === target[2]) self = true;
					ts.forEachChild(e, look);
				}
			};
			look(target[1]);
			if (frames && (LEAD_NAME.test(target[0]) || (bounded && self))) hit('lead-shape', n);
		}
		// texts the core builds
		if (!folded.has(n) && (ts.isStringLiteralLike(n) || ts.isTemplateExpression(n) || ts.isBinaryExpression(n) || ts.isCallExpression(n) || ts.isConditionalExpression(n))) {
			const f = fold(n);
			if (f) {
				const mark = (x) => { folded.add(x); ts.forEachChild(x, mark); };
				ts.forEachChild(n, mark);
				if (f.some((s) => CLOCK_TEXT.test(s) || HUD_TEXT.test(s))) hit('text-shape', n);
			}
		}
		// the being's moods; numbers that clone a Rust constant
		if (ts.isStringLiteralLike(n) && ctx.moods.has(n.text)) hit('mood-literal', n);
		if (ts.isNumericLiteral(n) && !ts.isArrayLiteralExpression(n.parent)) {
			const v = num(n.text);
			if (distinctive(v) && ctx.consts.has(v)) hit('twin-const', n);
		}
		ts.forEachChild(n, visit);
	};
	visit(sf);
	return hits;
}

/**
 * One file's text → its hits per rule (escaped ones not counted), the hits with their lines and code, the core-only-ok lines
 * (with a ticket) and the ones without a ticket. `ctx` = the deck's vocabulary (emptyContext() = the v1 patterns only).
 */
export function scan(text, ctx = emptyContext(), { svelte = false } = {}) {
	const lines = text.split(/\r?\n/);
	const state = { block: false };
	const split_ = lines.map((l) => split(l, state));
	const families = lines.map(() => new Set());
	const raw = [];
	split_.forEach(({ code, comment }, i) => {
		for (const [name, { re }] of Object.entries(PATTERNS)) {
			const n = (code.match(re) ?? []).length;
			if (!n) continue;
			families[i].add(name);
			for (let k = 0; k < n; k++) raw.push({ rule: name, line: i });
		}
		// a twin named in a comment needs a ticket in the comment (this line or its neighbours in the same block)
		if (TWIN_PHRASE.test(comment) && !/core-only-ok/.test(comment)) {
			let ticket = false;
			for (const d of [0, -1, 1, -2, 2, -3, 3]) {
				const j = i + d;
				if (j < 0 || j >= lines.length) continue;
				if (d !== 0 && split_[j].code.trim()) continue;
				if (/#\d+/.test(split_[j].comment)) ticket = true;
			}
			if (!ticket) raw.push({ rule: 'twin-comment', line: i });
		}
	});
	if (ctx.kinds.size || ctx.moods.size || ctx.consts.size) {
		for (const [block, first] of scripts(text, svelte)) {
			let found;
			try {
				found = astHits(block, ctx);
			} catch {
				continue;
			}
			for (const h of found) {
				const line = h.line + first;
				const family = RULES[h.rule][0];
				// a line inside a comment is no code (a JSDoc example); a v1 family hit on the line already counts it
				if (!split_[line] || !split_[line].code.trim()) continue;
				if (family && families[line]?.has(family)) continue;
				// a number on a line a v1 pattern already counts (a MIDI byte, a clamp's bound) is no second hit
				if (h.rule === 'twin-const' && families[line]?.size) continue;
				raw.push({ rule: h.rule, line });
			}
		}
	}
	const counts = {};
	const hits = [];
	const excused = [];
	const bad = [];
	const badLines = new Set();
	for (const h of raw.sort((a, b) => a.line - b.line)) {
		const esc = ESCAPE.exec(lines[h.line]) ?? (h.line > 0 ? ESCAPE.exec(lines[h.line - 1]) : null);
		if (esc && /#\d+/.test(esc[1])) {
			excused.push({ line: h.line + 1, pattern: h.rule, reason: esc[1].trim() });
			continue;
		}
		if (esc && !badLines.has(h.line)) {
			badLines.add(h.line);
			bad.push({ line: h.line + 1, reason: esc[1].trim() });
		}
		counts[h.rule] = (counts[h.rule] ?? 0) + 1;
		hits.push({ rule: h.rule, line: h.line + 1, code: split_[h.line].code.trim() });
	}
	return { counts, hits, excused, bad };
}

/** One file's size: lines, functions (declarations, methods, arrows), `Math.` calls (its budget's measures) */
export function measure(text, { svelte = false } = {}) {
	let functions = 0;
	let math = 0;
	for (const [block] of scripts(text, svelte)) {
		let sf;
		try {
			sf = ts.createSourceFile('x.ts', block, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
		} catch {
			continue;
		}
		const visit = (n) => {
			if (ts.isFunctionLike(n) && (ts.isFunctionDeclaration(n) || ts.isMethodDeclaration(n) || ts.isArrowFunction(n) || ts.isFunctionExpression(n) || ts.isGetAccessor(n) || ts.isSetAccessor(n) || ts.isConstructorDeclaration(n))) functions++;
			if (ts.isPropertyAccessExpression(n) && ts.isIdentifier(n.expression) && n.expression.text === 'Math') math++;
			ts.forEachChild(n, visit);
		};
		visit(sf);
	}
	return { lines: text.split(/\r?\n/).length, functions, math };
}

/** the module specifiers a file imports (static, dynamic, `export … from`) */
export function imports(text) {
	return [...text.matchAll(/(?:\bfrom\s*|\bimport\s*\(\s*|\bimport\s+)['"]([^'"]+)['"]/g)].map((m) => m[1]);
}

/** Rust sources → the constants (value → names) and numeric arrays (sorted value sets) the twin rule compares with */
export function rustNumbers(sources) {
	const consts = new Map();
	const arrays = [];
	const lit = /-?(?:0x[0-9a-f_]+|\d[\d_]*(?:\.\d[\d_]*)?(?:e-?\d+)?)(?:_?(?:f32|f64|u8|u16|u32|u64|usize|i32|i64))?/i;
	for (const [file, text] of sources) {
		for (const m of text.matchAll(/\bconst\s+(\w+)\s*:\s*[\w\s;\[\]]+?=\s*(-?[\d_][\w.]*)\s*;/g)) {
			const v = num(m[2].replace(/_?(?:f32|f64|u8|u16|u32|u64|usize|i32|i64)$/, ''));
			if (distinctive(v)) consts.set(v, [...(consts.get(v) ?? []), `${file} ${m[1]}`]);
		}
		for (const m of text.matchAll(/\[((?:\s*-?[\d][\w.]*\s*,){2,}\s*-?[\d][\w.]*\s*,?\s*)\]/g)) {
			const vs = m[1].split(',').map((s) => s.trim()).filter(Boolean);
			if (!vs.every((s) => lit.test(s))) continue;
			const values = vs.map((s) => num(s.replace(/_?(?:f32|f64|u8|u16|u32|u64|usize|i32|i64)$/, '')));
			if (values.some((v) => !Number.isFinite(v)) || !tableLike(values)) continue;
			arrays.push({ key: [...new Set(values)].sort((a, b) => a - b).join(','), file });
		}
	}
	return { consts, arrays };
}
