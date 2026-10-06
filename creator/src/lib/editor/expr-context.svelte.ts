// What the creator's expression fields share (agents/EXPRESSIONS.md step 4): the toon's story variables (the chapter
// editor publishes its toon facts – the builder offers them, the analyser warns about unknown ones), the sample reader
// the previews use (builder + slice canvas: one choice), and a jump request from the Problems panel (an `expr:*`
// problem: the field at that place focuses and selects the problem's range).
import type { SampleReaderId } from '$lib/editor/sample-readers.ts';

/** a field's place: the document (entity), the text's path and its language (null = not per language) */
export const fieldKey = (file: string, path: readonly (string | number)[], locale: string | null) => `${file}|${path.join('.')}|${locale ?? ''}`;

export interface ExprFocus {
	key: string;
	/** a string[] text's line (the field shows them joined by blank lines) */
	line?: number;
	start: number;
	end: number;
}

class ExprContext {
	/**
	 * the story variables there are, as written in this toon (`x`, `global.x`, `toptoy.x` – @scutoons/content
	 * variables.ts; sorted); null = not known (no facts loaded: unknown names aren't judged)
	 */
	variables = $state<string[] | null>(null);
	/** the toon being edited (a bare variable name is its own) */
	toon = $state('');
	/** every toon (id + name): the domains of the variable fields */
	toons = $state<{ id: string; name: string }[]>([]);
	/** the previews' sample reader; 'written' = the canvas shows texts as written */
	sample = $state<SampleReaderId | 'written'>('guest');
	/** a pending jump to a range (taken by the field it names) */
	focus = $state<ExprFocus | null>(null);

	setVariables(names: Iterable<string> | null | undefined) {
		const next = names ? [...new Set(names)].sort((a, b) => a.localeCompare(b)) : null;
		if (JSON.stringify(next) !== JSON.stringify(this.variables)) this.variables = next;
	}
}

export const exprContext = new ExprContext();
