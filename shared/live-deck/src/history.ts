// history.ts: deck-core's history rules (#687) – a document's saved versions as steps, a change in words, a revert –
// space-web's free functions as they are (as wasm, no GPU), JSON in and out. No rule here; the shapes: @scutoons/content.
import type { HistoryView } from '@scutoons/content';
import { spaceModule } from './space.ts';

/** space-web's `history_*` exports */
interface HistoryExports {
	history_steps(versions: string): string;
	history_describe(before: string, after: string): string;
	history_revert(before: string, after: string, current: string): string;
	history_back_to(versions: string, point: number, me: string): string;
}

const s = JSON.stringify;

/** the core's history rules from the wasm at `base` (the same module the deck loads: loaded once per page) */
export async function loadHistory(base: string): Promise<HistoryView> {
	const mod = (await spaceModule(base)) as unknown as HistoryExports;
	return {
		steps: (versions) => JSON.parse(mod.history_steps(s(versions))),
		describe: (before, after) => JSON.parse(mod.history_describe(s(before), s(after))),
		revert: (before, after, current) => JSON.parse(mod.history_revert(s(before), s(after), s(current))),
		backTo: (versions, point, me) => JSON.parse(mod.history_back_to(s(versions), point, me))
	};
}
