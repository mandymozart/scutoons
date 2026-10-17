// The number logic of the compact property controls (agents/research/EDITOR-INPUTS.md): typed maths, steps, rounding.
// No Svelte, no DOM – ScrubNumber and LinkedPair use it.

/** Keeps a value inside [min, max] (either may be missing). */
export function clamp(v: number, min?: number, max?: number): number {
	if (min !== undefined && v < min) return min;
	if (max !== undefined && v > max) return max;
	return v;
}

/** Rounds to the precision of `step` (0.01 → 2 decimals), so 0.1 + 0.2 shows as 0.3. */
export function roundToStep(v: number, step: number): number {
	const decimals = Math.max(0, Math.min(6, Math.ceil(-Math.log10(step) + 1e-9)));
	const f = 10 ** decimals;
	return Math.round(v * f) / f;
}

/** The step a key or drag uses: Shift ×10 (coarse), Alt ×0.1 (fine). */
export function stepFor(step: number, e: { shiftKey: boolean; altKey: boolean }): number {
	return e.shiftKey ? step * 10 : e.altKey ? step / 10 : step;
}

/**
 * Evaluates what was typed into a number field: a number, plain arithmetic (`12+3*2`, `(10-4)/2`), or an operation on
 * the old value (`+10`, `-5`, `*2`, `/2`). A leading minus followed by a number is a negative number, not "old − n",
 * unless written with a space (`- 5`) – like Figma. Returns null for anything else (no `eval`).
 */
export function evaluate(raw: string, old?: number): number | null {
	const text = raw.trim().replace(/,/g, '.');
	if (text === '') return null;
	let src = text;
	if (old !== undefined && /^([*/+]|-\s)/.test(text)) src = `${old}${text}`;
	const tokens = src.match(/\d*\.?\d+(?:e[+-]?\d+)?|[-+*/()]|\S/gi) ?? [];
	let i = 0;
	const peek = () => tokens[i];
	function expr(): number {
		let v = term();
		while (peek() === '+' || peek() === '-') v = tokens[i++] === '+' ? v + term() : v - term();
		return v;
	}
	function term(): number {
		let v = factor();
		while (peek() === '*' || peek() === '/') v = tokens[i++] === '*' ? v * factor() : v / factor();
		return v;
	}
	function factor(): number {
		const t = tokens[i++];
		if (t === '-') return -factor();
		if (t === '+') return factor();
		if (t === '(') {
			const v = expr();
			if (tokens[i++] !== ')') throw new Error('paren');
			return v;
		}
		const n = Number(t);
		if (t === undefined || !Number.isFinite(n)) throw new Error('token');
		return n;
	}
	try {
		const v = expr();
		return i === tokens.length && Number.isFinite(v) ? v : null;
	} catch {
		return null;
	}
}

/** How many steps a drag of `dx` CSS px makes: one step per `perStepRem` rem (the root font size from the document). */
export function dragSteps(dx: number, perStepRem = 0.25): number {
	const root = typeof document === 'undefined' ? 16 : parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	return dx / (perStepRem * root);
}
