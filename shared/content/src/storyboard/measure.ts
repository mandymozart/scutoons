// Reading measures of shots, VN groups and scenes (from scripts/storyboard/storyboard.mjs, #119): height, screens,
// words, boxes and the seconds of the rhythm model (STORYBOARD.md §6).

import { S, W, parseAspect, shotHeight } from './grid.ts';
import type { Shot, StoryboardDoc, StoryboardGroup } from './types.ts';
import { RHYTHM, readSeconds, words } from './vocab.ts';

/** every shot of a storyboard with its group, in order */
export const shotsOf = (sb: StoryboardDoc): { group: StoryboardGroup; shot: Shot }[] =>
	(sb.groups ?? []).flatMap((g) => (g.shots ?? []).map((shot) => ({ group: g, shot })));

export interface ShotMeasure {
	/** height on the 390-wide canvas (a loop360: one screen) */
	h: number;
	screens: number;
	words: number;
	boxes: number;
	/** looking, reading, scrolling – seconds */
	lookS: number;
	textS: number;
	scrollS: number;
	seconds: number;
	/** a panorama's width in screens of 390 (0 for others) */
	panoScreens: number;
}

/** reading measures of one shot: height on the canvas, screens, words, boxes, seconds (careful by default) */
export function measureShot(shot: Pick<Shot, 'aspect' | 'format' | 'dialog'>, look: number = RHYTHM.lookPerScreen.careful): ShotMeasure {
	const h = shotHeight(shot.aspect, shot.format);
	const screens = h / S;
	const boxes = (shot.dialog ?? []).map((d) => words(d.text));
	const n = boxes.reduce((a, b) => a + b, 0);
	const pano = shot.format === 'loop360' ? (parseAspect(shot.aspect)?.w ?? 1) / (parseAspect(shot.aspect)?.h ?? 1) : 0;
	// a panorama is one screen of scroll but is looked at across its width (screens of 390)
	const panoScreens = pano ? (pano * S) / W : 0;
	const lookS = look * (pano ? Math.max(1, panoScreens * 0.5) : screens);
	const textS = boxes.reduce((a, b) => a + readSeconds(b), 0);
	const scrollS = RHYTHM.scrollPerScreen * screens;
	return { h, screens, words: n, boxes: boxes.length, lookS, textS, scrollS, seconds: lookS + textS + scrollS, panoScreens };
}

export interface VnMeasure {
	messages: number;
	words: number;
	/** the longest run of messages without a choice / plate change */
	maxRun: number;
	seconds: number;
}

/** VN group: messages, words, seconds, longest run without a choice / plate change */
export function measureVn(group: Pick<StoryboardGroup, 'events'>): VnMeasure {
	const msgs = (group.events ?? []).filter((e) => e.type === 'showMessage');
	let run = 0, maxRun = 0;
	for (const e of group.events ?? []) {
		if (e.type === 'showMessage') run++;
		if (['showChoices', 'joinScene', 'leaveScene'].includes(e.type)) run = 0;
		maxRun = Math.max(maxRun, run);
	}
	const n = msgs.reduce((a, m) => a + words(m.params?.text), 0);
	return { messages: msgs.length, words: n, maxRun, seconds: msgs.length * (RHYTHM.perBox + RHYTHM.perClick) + n / RHYTHM.wordsPerSecond };
}

export interface SceneSummary {
	scene: number | undefined;
	heading: string | undefined;
	shots: number;
	screens: number;
	seconds: number;
	words: number;
}

/** summary of a set of storyboards: screens (a loop360 counts one), seconds, words, per scene and in total */
export function summarize(list: { sb: StoryboardDoc }[]): { scenes: SceneSummary[]; total: Omit<SceneSummary, 'scene' | 'heading'> } {
	const scenes = list.map(({ sb }) => {
		let screens = 0, seconds = 0, nWords = 0, shots = 0;
		for (const g of sb.groups ?? []) {
			for (const s of g.shots ?? []) {
				const m = measureShot(s);
				screens += s.format === 'loop360' ? 1 : m.screens;
				seconds += m.seconds;
				nWords += m.words;
				shots++;
			}
			if ((g.events ?? []).length) {
				const v = measureVn(g);
				seconds += v.seconds;
				nWords += v.words;
			}
		}
		return { scene: sb.source?.scene, heading: sb.source?.heading, shots, screens, seconds, words: nWords };
	});
	const total = scenes.reduce(
		(a, s) => ({ shots: a.shots + s.shots, screens: a.screens + s.screens, seconds: a.seconds + s.seconds, words: a.words + s.words }),
		{ shots: 0, screens: 0, seconds: 0, words: 0 }
	);
	return { scenes, total };
}
