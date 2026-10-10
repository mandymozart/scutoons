// The storyboard's texts in the SCUTOONS text presets (owner 2026-10-09: "the storyboard 1st throw also needs the
// design dogma from scutoons"; agents/DESIGN.md Part 2, creator/src/lib/editor/text-presets.ts). One mapping for the
// frames (core.js drawDialog) and the chapter draft (scripts/storyboard/compile.mjs), so the first throw looks like
// the reader and opens in the creator as the presets:
//   narration                → Narration      box, italic mono, left, middle, 4 % of the slice width
//   narration "25 EXT. …"    → Scene          box, italic mono in capitals
//   bubble (speech)          → Speech bubble  a drawn bubble (frame + tail), mono, centred – webtoon storyboard balloons
//   caption + lettering      → Onomatopoeia   no box, Bangers 15 %, white letters, ink outline 0.1 em, −7°
//   lettering + title: true  → the chapter title ('4. KAPITEL' / '„DEPRESSION“' – owner 2026-10-10, as in chapter 2):
//                              the Onomatopoeia look, straight, its own size
//   caption                  → Story          no box, mono, top
// Plain ESM (browser + Node).

const SCENE = /^\s*\d+[a-z]?\s+(INT|EXT|INT\.?\/EXT)\b/i;

/** the preset of a storyboard text */
export function presetOf(d) {
	if (d.font === 'lettering') return d.title ? 'title' : 'onomatopoeia';
	if (d.variant === 'bubble') return 'speech-bubble';
	if (d.variant === 'narration') return SCENE.test(String(d.text ?? '')) ? 'scene' : 'narration';
	return 'story';
}

/** the Dialog look fields of a preset (the chapter's own fields – Dialog in shared/content) */
export function presetStyle(preset, d = {}) {
	switch (preset) {
		case 'scene': return { variant: 'narration', font: 'mono', align: 'left', valign: 'middle', size: 4, uppercase: true };
		case 'narration': return { variant: 'narration', font: 'mono', align: 'left', valign: 'middle', size: 4 };
		case 'speech-bubble': return { variant: 'bubble', frame: true, font: 'mono', align: 'center', valign: 'middle', size: 4, tail: d.tail ?? 'bottom-left' };
		case 'onomatopoeia': return { variant: 'bubble', font: 'lettering', align: 'center', valign: 'middle', size: 15, color: '#ffffff', stroke: '#000000', strokeWidth: 0.1, rotation: -7 };
		case 'title': return { variant: 'bubble', font: 'lettering', align: 'center', valign: 'middle', size: d.size ?? 15, color: '#ffffff', stroke: '#000000', strokeWidth: 0.1, rotation: 0 };
		default: return { variant: 'caption', frame: false, font: 'mono', align: 'left', valign: 'top', size: 4 };
	}
}
