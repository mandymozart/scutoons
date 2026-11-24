// What the Publish dialog (lap 2 A10, RULES H18) decides from its list alone: which lines start ticked, All · None,
// what its one primary button says and whether a confirm comes before sending.

export interface ChoiceLine {
	id: string;
	/** this account submits it for review instead of publishing it */
	review: boolean;
	/** someone else's submit this account decides on (published as submitted): its reviews */
	decide?: readonly number[];
	/** the server refuses this key: never ticked */
	refused?: string;
}

/** every line ticked, but the ones the open page holds back and the ones the server refuses */
export const startTicks = (lines: ChoiceLine[], held: ReadonlySet<string>): Set<string> => new Set(lines.filter((l) => !held.has(l.id) && !l.refused).map((l) => l.id));

/** one line ticked or not */
export function toggled(ticked: ReadonlySet<string>, id: string, on: boolean, lines: readonly ChoiceLine[] = []): Set<string> {
	if (on && lines.some((l) => l.id === id && l.refused)) return new Set(ticked);
	const next = new Set(ticked);
	if (on) next.add(id);
	else next.delete(id);
	return next;
}

/** the primary button: "Submit {n} for review" as soon as a ticked line goes to review, else "Publish {n}" */
export const primaryKind = (lines: ChoiceLine[], ticked: ReadonlySet<string>): 'publish' | 'submit' => (lines.some((l) => ticked.has(l.id) && l.review) ? 'submit' : 'publish');

/** the ticked submits of others, decided here */
export const decisions = <L extends ChoiceLine>(lines: L[], ticked: ReadonlySet<string>): L[] => lines.filter((l) => l.decide && ticked.has(l.id));

/** a confirm before sending: the open page lists errors, or someone else's submit goes live (its author gets a note) */
export const needsConfirm = (lines: ChoiceLine[], ticked: ReadonlySet<string>, errors: number): boolean => errors > 0 || decisions(lines, ticked).length > 0;
