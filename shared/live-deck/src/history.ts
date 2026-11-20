// history.ts: deck-core's history rules (#687) from the wasm the page loads (the same module the deck loads: once per page,
// as wasm, no GPU). No rule here; the mapping is @scutoons/content's history-view.ts.
import { historyView, type HistoryExports, type HistoryView } from '@scutoons/content';
import { spaceModule } from './space.ts';

/** the core's history rules from the wasm at `base` */
export async function loadHistory(base: string): Promise<HistoryView> {
	return historyView((await spaceModule(base)) as unknown as HistoryExports);
}
