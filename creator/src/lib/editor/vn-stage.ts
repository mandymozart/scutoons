// The stage on an event list (owner 2026-10-13: "warn about a character used before being added to the scene"): the
// validator's own walk (@scutoons/content stageProblems) over the list's group, as speakers per event index, and the
// quick fix – a joinScene for the speaker right before the event (a slice event: with the event's trigger).
import { clone, getAt, stageProblems, type Path } from '@scutoons/content';
import { template } from '$lib/editor/vn-forms.ts';

/** the group an event list belongs to: its script (`…/events`) or one of its slices (`…/slices/<n>/events`) */
export function groupPathOf(eventsPath: Path): Path {
	return eventsPath.length >= 3 && eventsPath[eventsPath.length - 3] === 'slices' ? eventsPath.slice(0, -3) : eventsPath.slice(0, -1);
}

/** the list's speakers who aren't on stage: event index → the character as written in the event */
export function offStage(file: unknown, eventsPath: Path): Map<number, string> {
	const groupPath = groupPathOf(eventsPath);
	const list = eventsPath.slice(groupPath.length);
	const out = new Map<number, string>();
	for (const s of stageProblems(getAt(file, groupPath))) {
		if (s.kind !== 'speaks' || s.event.length !== list.length + 1 || !list.every((k, i) => s.event[i] === k)) continue;
		const index = Number(s.event[list.length]);
		const event = (getAt(file, eventsPath) as any[] | undefined)?.[index];
		out.set(index, String(event?.params?.character ?? s.id));
	}
	return out;
}

/** a joinScene for event `index`'s speaker, inserted before it (the caller shifts its indices from `index` on) */
export function joinBefore(file: unknown, eventsPath: Path, index: number): boolean {
	const list = getAt(file, eventsPath) as any[] | undefined;
	const speaker = list?.[index]?.params;
	if (!Array.isArray(list) || !speaker?.character) return false;
	const join = template('joinScene', { firstCharacter: speaker.character, events: list, pointNames: [] });
	join.params.characters[0].persona = speaker.persona ?? join.params.characters[0].persona;
	if (list[index].trigger) join.trigger = clone(list[index].trigger);
	list.splice(index, 0, join);
	return true;
}
