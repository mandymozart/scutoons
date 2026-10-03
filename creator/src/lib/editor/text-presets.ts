import type { Dialog } from '@scutoons/content';

/**
 * The text presets (owner 2026-10-03): a text on a slice is one thing, its look one of these. "+ Add text" offers them
 * with a preview; the inspector applies one to an existing text (its words and position stay). Everything is in the
 * existing Dialog fields – no new data: narration = the italic box, speech = the box, story = bare text at the top,
 * onomatopoeia = bare lettering with an outline.
 */
export type TextPresetId = 'narration' | 'speech' | 'onomatopoeia' | 'story';

export interface TextPreset {
	id: TextPresetId;
	/** the look: every field a preset owns (others are cleared when it is applied) */
	style: Omit<Dialog, 'rect' | 'text' | 'id'>;
	/** where a new text of this preset goes (% of the slice; right = 100 − x − width) */
	rect: Dialog['rect'];
}

export const TEXT_PRESETS: TextPreset[] = [
	{
		id: 'narration',
		style: { variant: 'narration', font: 'mono', align: 'left', valign: 'middle', size: 4 },
		rect: { x: 10, y: 10, width: 80, height: 12 }
	},
	{
		id: 'speech',
		style: { variant: 'caption', font: 'mono', align: 'left', valign: 'middle', size: 4 },
		rect: { x: 10, y: 10, width: 80, height: 12 }
	},
	{
		id: 'onomatopoeia',
		style: { variant: 'bubble', font: 'lettering', align: 'center', valign: 'middle', size: 15, color: '#ffffff', stroke: '#000000', strokeWidth: 0.1, rotation: -7 },
		rect: { x: 15, y: 40, width: 70 }
	},
	{
		id: 'story',
		style: { variant: 'caption', frame: false, font: 'mono', align: 'left', valign: 'top', size: 4 },
		rect: { x: 10, y: 10, width: 80, height: 20 }
	}
];

/** the look fields a preset sets or clears (the text's words, id, position and size stay) */
const LOOK_FIELDS = ['variant', 'frame', 'shape', 'tail', 'font', 'align', 'valign', 'size', 'weight', 'uppercase', 'color', 'stroke', 'strokeWidth', 'rotation', 'scale'] as const;

/** applies a preset's look to a text (in place) */
export function applyTextPreset(dialog: Record<string, unknown>, preset: TextPreset) {
	for (const key of LOOK_FIELDS) delete dialog[key];
	Object.assign(dialog, structuredClone(preset.style));
}

/** the preset a text's look matches exactly, if any */
export function matchTextPreset(dialog: Record<string, unknown> | null | undefined): TextPresetId | null {
	if (!dialog) return null;
	const found = TEXT_PRESETS.find((p) =>
		LOOK_FIELDS.every((key) => (dialog[key] ?? undefined) === ((p.style as Record<string, unknown>)[key] ?? undefined))
	);
	return found?.id ?? null;
}
