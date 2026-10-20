// The shot list of a chapter's storyboards (from scripts/storyboard/shotlist.mjs, #119), grouped by place and by what
// each shot wants (photo / unreal / direct / generated / art): the sheet for matching the Figma selection, for planning
// a shooting and – for the gaps – for prompting.

import { measureShot, shotsOf } from './measure.ts';
import type { StoryboardDoc } from './types.ts';
import { textOf } from './vocab.ts';

export interface ShotRow {
	scene: number | undefined;
	id: string;
	place: string;
	time: string;
	/** source.want, '?' when missing */
	want: string;
	format: string;
	/** "390 × 1170" or "6.5 screens wide" */
	px: string;
	framing: string;
	who: string;
	action: string;
}

/** the order of the want table */
export const WANTS = ['photo', 'direct', 'unreal', 'generated', 'art', '?'] as const;

/** one row per shot, in scene order */
export function shotRows(scenes: StoryboardDoc[]): ShotRow[] {
	const rows: ShotRow[] = [];
	for (const sb of scenes) {
		const places = Object.fromEntries((sb.locations ?? []).map((l) => [l.ref, l]));
		const cast = Object.fromEntries((sb.cast ?? []).map((c) => [c.ref, c]));
		for (const { shot } of shotsOf(sb)) {
			const loc = shot.location !== undefined ? places[shot.location] : undefined;
			const m = measureShot(shot);
			rows.push({
				scene: sb.source?.scene, id: shot.id, place: loc?.place ?? loc?.new ?? loc?.ref ?? '–', time: loc?.time ?? '',
				want: shot.source?.want ?? '?', format: `${shot.format} ${shot.aspect}`, px: shot.format === 'loop360' ? `${m.panoScreens.toFixed(1)} screens wide` : `390 × ${m.h}`,
				framing: [shot.shot?.size, shot.shot?.angle, shot.projection !== 'perspective' ? shot.projection : ''].filter(Boolean).join(' '),
				who: (shot.figures ?? []).map((f) => `${cast[f.cast]?.new ?? f.cast}${f.mood ? ` (${f.mood})` : ''}`).join(', '),
				action: textOf(shot.action)
			});
		}
	}
	return rows;
}

/** the shots per want, in WANTS order, without the empty ones */
export const wantCounts = (rows: ShotRow[]): [string, number][] =>
	WANTS.map((w): [string, number] => [w, rows.filter((r) => r.want === w).length]).filter(([, n]) => n > 0);

/** the Markdown sheet: the want table, then a table per place */
export function shotlistMarkdown(name: string, rows: ShotRow[]): string {
	const md = [`# Shot list ${name} – ${rows.length} shots`, '',
		'What each shot needs, by place. Match against the Figma selection on the chapter page; the shooting pages only for gaps.',
		'`want`: photo = from a shooting · direct = to be photographed · unreal = render · generated = prompt it (Luma, manual) · art = drawn / graphic.', '',
		'| want | shots |', '|---|---|', ...WANTS.map((w) => `| ${w} | ${rows.filter((r) => r.want === w).length} |`).filter((l) => !l.endsWith('| 0 |')), ''];
	for (const place of [...new Set(rows.map((r) => r.place))]) {
		const rs = rows.filter((r) => r.place === place);
		md.push(`## ${place} (${rs.length})`, '', '| shot | want | format | size | framing | who | what the image shows |', '|---|---|---|---|---|---|---|');
		for (const r of rs) md.push(`| ${r.id} | ${r.want} | ${r.format} | ${r.px} | ${r.framing} | ${r.who} | ${r.action.replace(/\|/g, '/')} |`);
		md.push('');
	}
	return md.join('\n');
}
