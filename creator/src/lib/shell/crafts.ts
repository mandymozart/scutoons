// Reading the member's crafts so the creator never depends on them (pure; tests: test/shell.test.ts). The API may not
// answer yet – code goes live before its migration – then the member has no crafts: every workspace, no first-login question.
import type { Craft, MemberCrafts } from '@scutoons/api';

export interface CraftsState {
	list: Craft[];
	mine: MemberCrafts | null;
}

export const NO_CRAFTS: CraftsState = { list: [], mine: null };

/** both answers, or NO_CRAFTS when either fails (404 before the migration, 500, offline) or comes back malformed */
export async function readCrafts(list: () => Promise<{ crafts?: unknown }>, mine: () => Promise<unknown>): Promise<CraftsState> {
	try {
		const [l, me] = await Promise.all([list(), mine()]);
		const crafts = Array.isArray(l?.crafts) ? (l.crafts as Craft[]) : null;
		const m = me as Partial<MemberCrafts> | null;
		if (!crafts || !m || !Array.isArray(m.crafts) || !Array.isArray(m.workspaces)) return NO_CRAFTS;
		return { list: crafts, mine: { crafts: m.crafts, invited: Array.isArray(m.invited) ? m.invited : [], workspaces: m.workspaces, ask: m.ask === true, can_edit: m.can_edit === true } };
	} catch {
		return NO_CRAFTS;
	}
}

/** the first-login question: only when the API says it is open and there are crafts to pick */
export const asksCrafts = (state: CraftsState): boolean => !!state.mine?.ask && state.list.length > 0;

/** the crafts whose bar carries the text language beside the workspace row (lap 2 A9: the translator's bar) */
const TEXT_LANGUAGE_CRAFTS = ['translator'];

/** the member's bar carries the text language */
export const showsTextLanguage = (mine: MemberCrafts | null): boolean => !!mine?.crafts.some((c) => TEXT_LANGUAGE_CRAFTS.includes(c));
