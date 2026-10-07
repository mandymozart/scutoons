import type { ExprResolver, JumpToTarget, VNEvent } from '@scutoons/content';
import { cleanText, READER_NAME_MAX, readerKnown, readerName } from '$lib/reader-name.svelte.ts';
import type { GameContext } from '$lib/contexts/game.svelte';
import type { VisualNovelContext } from '$lib/contexts/visual-novel.svelte';
import { getCurrentReaderExpressions } from '$lib/expressions/resolver.svelte.ts';
import { executeVariableAction, giveStoryItem, takeStoryItem } from './variable-actions.ts';
import { runEvents, type RunnerEnv, type RunResult } from './script-runner.ts';

/**
 * The one VN script runner (CUTOVER 10.27, owner 2026-10-03: logic stays out of the views): the group's script
 * (`interactions/visual-novel.svelte`) and slice / placed-item triggers (`event-processor.ts`) both run events here. The
 * logic is `script-runner.ts` (pure, tested in node with the box of box-state.ts); this file wires it to the viewer –
 * the reader's session and name, the expression resolver, item grants, the page's jump places.
 */
export interface RunnerDeps {
	vn: VisualNovelContext;
	game: GameContext;
	/** go to another page (group / chapter); the run ends */
	navigate: (url: string) => void;
	/** the page went away (the group component was destroyed) */
	cancelled?: () => boolean;
	/** the group the reader is on (a random jump never picks it) */
	currentLabel?: () => string | null;
	/** the group of the page whose script / slice events run (its text boxes, linked groups share a page) */
	scope?: string;
	/**
	 * where the events come from, for their texts' identity (visits of `{a|b|c}`): `script` = the group's script (default),
	 * else e.g. a slice trigger / a placed item – a message is `<group label>/<origin>/<event index>`
	 */
	origin?: string;
	/** a page script: it pauses while a slice-trigger run holds the box */
	yieldToRuns?: boolean;
}

export type { RunResult };

/** the viewer's reader around a run (the session, the name, `{…}` and expression conditions, item grants) */
export function readerEnv(game: GameContext): RunnerEnv {
	// messages and choice labels: `{…}` answered for the reader (agents/EXPRESSIONS.md) – each showing counts a visit;
	// without the toon's expressions (a lone preview) only the story variables
	const expressions = getCurrentReaderExpressions();
	const conditions: ExprResolver | undefined = expressions ? { name: expressions.name, call: expressions.call } : undefined;
	return {
		readerKnown: () => readerKnown(),
		setReaderName: (name) => readerName.set(name),
		readerNameMax: READER_NAME_MAX,
		...(expressions ? { evaluate: (source: string, at: string) => expressions.evaluate(source, at) } : {}),
		...(conditions ? { conditions } : {}),
		setVariable: (action) => void executeVariableAction(action as never, game),
		giveItem: (item, qty) => void giveStoryItem(item, qty),
		takeItem: (take) => void takeStoryItem(take as never, game),
		cleanText
	};
}

export function runScript(events: readonly VNEvent[], start: number, deps: RunnerDeps): Promise<RunResult> {
	return runEvents(events, start, {
		...deps,
		env: readerEnv(deps.game),
		jumpProblem: (target: JumpToTarget) => deps.vn.jumpProblem(target)
	});
}
