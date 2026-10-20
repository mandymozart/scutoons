// Chapters 4–5's storyboard shots onto whole slices (#111; agents/research/SLICE-GRID.md §1, §4; owner 2026-10-20 Q5:
// "we do it now. layouting in figma hasnt started for sure."): the changes for `save_draft` on
// storyboard:<toon>:<chapter> – one `set` of `aspect` per viewport / vertical shot that isn't whole slices. A one-off:
// it goes once the owner has confirmed ch4–5's drafts. ch3 is never rewritten (SLICE-GRID §6).
import { aspectSlices } from '@scutoons/content/storyboard';

/** a viewport is one slice */
export const VIEWPORT = '1:1.6';
/** vertical moves fixed by SLICE-GRID §4: 1:3 (1.88 slices) → 2 slices; 1:4 (2.5 slices) is picked per shot */
export const VERTICAL: Record<string, string> = { '1:3': '1:3.2' };
export const PICKED = '1:4';
export const PICKS = ['1:3.2', '1:4.8'];

export type Path = (string | number)[];
export interface Op {
	op: 'set';
	path: Path;
	value: string;
}
export interface Change {
	id: string;
	from: string;
	to: string;
	picked: boolean;
}
export interface Plan {
	ops: Op[];
	changes: Change[];
	/** off-grid shots no rule covers – left for the owner (the validator keeps warning) */
	left: { id: string; format: string; aspect: string }[];
}

interface Shot {
	id?: string;
	format?: string;
	aspect?: string;
}
interface Doc {
	scenes?: { groups?: { shots?: Shot[] }[] }[];
}

/**
 * The plan for one chapter document. `picks` = shot id → 1:3.2 | 1:4.8 for the shots at 1:4; a 1:4 shot without a
 * valid pick throws (nothing is guessed here). Whole-slice shots, strips and panoramas stay as they are, so a second
 * run on the result plans nothing.
 */
export function sliceOps(doc: Doc, picks: Record<string, string>): Plan {
	const plan: Plan = { ops: [], changes: [], left: [] };
	(doc.scenes ?? []).forEach((scene, si) =>
		(scene.groups ?? []).forEach((group, gi) =>
			(group.shots ?? []).forEach((shot, hi) => {
				const { id = `?${si}.${gi}.${hi}`, format = '', aspect = '' } = shot;
				if ((format !== 'viewport' && format !== 'vertical') || aspectSlices(aspect)) return;
				let to: string | undefined;
				if (format === 'viewport') to = VIEWPORT;
				else if (aspect === PICKED) {
					to = picks[id];
					if (!PICKS.includes(to ?? '')) throw new Error(`${id}: vertical ${PICKED} needs a pick, ${PICKS.join(' or ')} (got ${to ?? 'none'})`);
				} else to = VERTICAL[aspect];
				if (!to) return void plan.left.push({ id, format, aspect });
				plan.ops.push({ op: 'set', path: ['scenes', si, 'groups', gi, 'shots', hi, 'aspect'], value: to });
				plan.changes.push({ id, from: aspect, to, picked: aspect === PICKED });
			})
		)
	);
	return plan;
}

/** picks naming no 1:4 shot of these plans (a typo, or a shot that moved) */
export function unusedPicks(plans: Plan[], picks: Record<string, string>): string[] {
	const used = new Set(plans.flatMap((p) => p.changes.filter((c) => c.picked).map((c) => c.id)));
	return Object.keys(picks).filter((id) => !used.has(id));
}
