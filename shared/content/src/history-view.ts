// history-view.ts: deck-core's history rules (#687) as space-web's free functions hand them over – JSON in and out. No rule
// here; the shapes: history.ts. Shared by the browser (live-deck) and the MCP (Node).
import type { HistoryView } from './history.ts';

/** space-web's `history_*` exports */
export interface HistoryExports {
	history_steps(versions: string): string;
	history_describe(before: string, after: string): string;
	history_revert(before: string, after: string, current: string): string;
	history_back_to(versions: string, point: number, me: string): string;
}

const s = JSON.stringify;

export function historyView(mod: HistoryExports): HistoryView {
	return {
		steps: (versions) => JSON.parse(mod.history_steps(s(versions))),
		describe: (before, after) => JSON.parse(mod.history_describe(s(before), s(after))),
		revert: (before, after, current) => JSON.parse(mod.history_revert(s(before), s(after), s(current))),
		backTo: (versions, point, me) => JSON.parse(mod.history_back_to(s(versions), point, me))
	};
}
