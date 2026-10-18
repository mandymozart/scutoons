import { defaultTail, dialogTail, freeTailFrom, isFreeTail, resolveText, tailKindOf, type Dialog, type TailBox, type TailKind } from '@scutoons/content';

/**
 * The text presets (owner 2026-10-03): a text on a slice is one thing, its look one of these. "+ Add text" offers them
 * with a preview; the inspector applies one to an existing text (its words and position stay). Everything is in the
 * existing Dialog fields – no new data: narration = the italic box, speech = the box, story = bare text at the top,
 * onomatopoeia = bare lettering with an outline, scene = the italic box in capitals ("19 EXT. POOL"), lyrics = the box
 * with musical notes on its border (a song line, owner 2026-10-17). Speech and thought (owner 2026-10-18) are drawn
 * bubbles (the same paper box) with a free tail – the speech wedge or the thought circles (dialog-tail.ts); the tail's
 * kind belongs to the look, its tip and base to the text (they stay when the look changes between the two).
 */
export type TextPresetId = 'narration' | 'speech' | 'thought' | 'onomatopoeia' | 'story' | 'scene' | 'lyrics';

export interface TextPreset {
	id: TextPresetId;
	/** the look: every field a preset owns (others are cleared when it is applied) */
	style: Omit<Dialog, 'rect' | 'text' | 'id'>;
	/** where a new text of this preset goes (% of the slice; right = 100 − x − width) */
	rect: Dialog['rect'];
	/** a drawn bubble's tail kind: a new text gets a default tail of it (`defaultTail`), a switched one keeps its tip */
	tail?: TailKind;
}

/** the drawn speech bubble's look (speech and thought share it, only the tail differs) */
const BUBBLE = { variant: 'bubble', frame: true, font: 'mono', align: 'left', valign: 'middle', size: 4 } as const;

export const TEXT_PRESETS: TextPreset[] = [
	{
		id: 'narration',
		style: { variant: 'narration', font: 'mono', align: 'left', valign: 'middle', size: 4 },
		rect: { x: 10, y: 10, width: 80, height: 12 }
	},
	{
		id: 'speech',
		style: { ...BUBBLE },
		rect: { x: 10, y: 10, width: 80, height: 12 },
		tail: 'speech'
	},
	{
		id: 'thought',
		style: { ...BUBBLE },
		rect: { x: 10, y: 10, width: 80, height: 12 },
		tail: 'thought'
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
	},
	{
		id: 'scene',
		style: { variant: 'narration', font: 'mono', align: 'left', valign: 'middle', size: 4, uppercase: true },
		rect: { x: 10, y: 10, width: 80, height: 8 }
	},
	{
		id: 'lyrics',
		style: { variant: 'lyrics', font: 'mono', align: 'left', valign: 'middle', size: 4 },
		rect: { x: 10, y: 10, width: 80, height: 12 }
	}
];

/** the look fields a preset sets or clears (the text's words, id, position and size stay) */
const LOOK_FIELDS = ['variant', 'frame', 'shape', 'tail', 'font', 'align', 'valign', 'size', 'weight', 'uppercase', 'color', 'stroke', 'strokeWidth', 'letterSpacing', 'lineHeight', 'rotation', 'scale'] as const;

/**
 * applies a preset's look to a text (in place). A preset with a tail: the text's free tail keeps its tip and base (only
 * the kind changes), an old fixed tail becomes a free one where it was, else a default tail for the drawn box `box`
 * (% of the slice; none = its rect) – `ratio` = the slice's height / width.
 */
export function applyTextPreset(dialog: Record<string, unknown>, preset: TextPreset, box?: TailBox, ratio?: number) {
	const before = dialogTail(dialog as unknown as Dialog);
	for (const key of LOOK_FIELDS) delete dialog[key];
	Object.assign(dialog, structuredClone(preset.style));
	if (!preset.tail) return;
	const rect = dialog.rect as Dialog['rect'] | undefined;
	const at: TailBox = box ?? { left: rect?.x ?? 10, top: rect?.y ?? 10, width: rect?.width ?? 40, height: rect?.height ?? 10 };
	dialog.tail = isFreeTail(before)
		? { ...structuredClone(before), kind: preset.tail }
		: typeof before === 'string'
			? freeTailFrom(before, at, (dialog.size as number) ?? 4, ratio, preset.tail)
			: defaultTail(preset.tail, at, ratio);
}

/** the preset a text's look matches exactly, if any (a tail by its kind: the tip and base are the text's own) */
export function matchTextPreset(dialog: Record<string, unknown> | null | undefined): TextPresetId | null {
	if (!dialog) return null;
	const kind = tailKindOf(dialog as unknown as Dialog);
	const found = TEXT_PRESETS.find((p) =>
		LOOK_FIELDS.every((key) =>
			key === 'tail'
				? (p.tail ?? 'none') === kind
				: (dialog[key] ?? undefined) === ((p.style as Record<string, unknown>)[key] ?? undefined)
		)
	);
	return found?.id ?? null;
}

/**
 * Chapter 1's texts (owner 2026-10-03: convert them): the old look (variant + mask over baked text) → a preset. Decided on
 * the source language's text, applied in every language. Scene headings ("2 INT. SCHLAFZIMMER - NACHT") → Scene; caption →
 * Speech; narration → Narration; bare texts (speech-bubble variant, unframed) → Story. Position and words stay; the
 * mask goes (the art is re-imported text-free).
 */
export function legacyPreset(dialog: Record<string, any>): TextPreset {
	const id = (presetId: TextPresetId) => TEXT_PRESETS.find((p) => p.id === presetId)!;
	const text = resolveText<unknown>(dialog.text, []);
	const first = String(Array.isArray(text) ? text[0] : (text ?? '')).trim();
	if (/^\d+\s+(INT|EXT|INT\.\/EXT)\b|^\d+\s+[A-ZÄÖÜ]{4,}/.test(first) && first === first.toUpperCase()) return id('scene');
	const variant = dialog.variant ?? 'bubble';
	if (variant === 'bubble' || dialog.frame === false) return id('story');
	return variant === 'narration' ? id('narration') : id('speech');
}

/** the dialog arrays of a document (paths), e.g. groups.0.slices.3.dialog */
function dialogPaths(node: unknown, path: (string | number)[] = [], out: (string | number)[][] = []) {
	if (Array.isArray(node)) node.forEach((v, i) => dialogPaths(v, [...path, i], out));
	else if (node && typeof node === 'object') {
		for (const [k, v] of Object.entries(node)) {
			if (k === 'dialog' && Array.isArray(v)) out.push([...path, k]);
			else dialogPaths(v, [...path, k], out);
		}
	}
	return out;
}
const at = (node: any, path: (string | number)[]) => path.reduce((n, k) => (n == null ? n : n[k]), node);

/** texts that still use the old look (a mask) in a document */
export function legacyTextCount(file: unknown): number {
	return dialogPaths(file).reduce((n, p) => n + (at(file, p) as any[]).filter((d) => d && d.mask !== undefined).length, 0);
}

/** which masked text becomes which preset, decided on the source language's document (before anything changes) */
export function legacyPlan(source: unknown): { path: (string | number)[]; preset: TextPreset }[] {
	return dialogPaths(source).flatMap((p) =>
		(at(source, p) as any[]).flatMap((d, i) => (d && d.mask !== undefined ? [{ path: [...p, i], preset: legacyPreset(d) }] : []))
	);
}

/** applies the plan to one language's document: the preset's look, no mask */
export function applyLegacyPlan(file: unknown, plan: ReturnType<typeof legacyPlan>) {
	for (const { path, preset } of plan) {
		const d = at(file, path);
		if (!d || typeof d !== 'object') continue;
		applyTextPreset(d, preset);
		delete d.mask;
		delete d.maskShape;
		delete d.pad;
	}
}
