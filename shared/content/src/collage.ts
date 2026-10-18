// Collage pieces – vectors, character pictures and images placed on slices and scores (agents/research/COLLAGE.md §2,
// owner 2026-10-17: "add these vector graphics on them and blend them … our avatars and portraits and full … all of
// the available blending modes … drop shadow … outlines, color fill gradients … blurring with a start and end point").
// ONE piece shape for both hosts: a slice's `pieces[]` (content.ts, % of the picture) and a score's `collage`
// (containers.ts, anchored to a bar + staff). Colour is never stored on a shape: a colouration is a named look
// (`COLLAGE_LOOKS`) a piece names, plus its own values over it (`style`). Vectors are never distorted: only the width is
// stored, the height follows the file's `ratio`.
import { ANCHORS, isAnchor, type Anchor } from './anchor.ts';
import { CHARACTER_IMAGE_VARIANTS, type CharacterImageVariant } from './characters.ts';

/** the 16 CSS blend modes – identical in Chromium, Firefox, WebKit on an HTML box (never on SVG children) */
export const BLEND_MODES = ['normal', 'multiply', 'screen', 'overlay', 'darken', 'lighten', 'color-dodge', 'color-burn',
	'hard-light', 'soft-light', 'difference', 'exclusion', 'hue', 'saturation', 'color', 'luminosity'] as const;
export type BlendMode = (typeof BLEND_MODES)[number];

/**
 * #rgb | #rrggbb – it reaches readers' CSS as written, so nothing else is a colour (no url(), no `;`)
 * @pattern ^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6})$
 */
export type PieceColor = string;

/** % of the piece's own box (0 0 top-left … 100 100 bottom-right): moves, scales, turns with the piece */
export interface PiecePoint {
	x: number;
	y: number;
}
export interface GradientStop {
	/** % along the line, ascending */
	at: number;
	color: PieceColor;
	/** 0…1 */
	opacity?: number;
}
/** one linear gradient across the whole silhouette */
export interface PieceGradient {
	/** CSS degrees, 0 = to the top; default 180 */
	angle?: number;
	/** at least 2 */
	stops: GradientStop[];
}
/** lengths in % of the piece's width (the same look at every size) */
export interface PieceOutline {
	color: PieceColor;
	width: number;
}
/** offsets and blur in % of the piece's width */
export interface PieceShadow {
	color: PieceColor;
	x: number;
	y: number;
	blur: number;
	/** 0…1, default 1 */
	opacity?: number;
}
/** in % of the piece's width; even, or with `from` + `to`: sharp at `from`, `amount` from `to` on (progressive) */
export interface PieceBlur {
	amount: number;
	from?: PiecePoint;
	to?: PiecePoint;
}

/** a colouration – a look's values, or a piece's own over its look (a key replaces it whole; null = off) */
export interface PieceStyle {
	blend?: BlendMode;
	/** 0…1 */
	opacity?: number;
	/** the file's silhouette (its alpha) in a colour / gradient; none = as drawn */
	fill?: PieceColor | PieceGradient | null;
	outline?: PieceOutline | null;
	shadow?: PieceShadow | null;
	blur?: PieceBlur | null;
}
/** a named colouration (tonight the ten COLLAGE_LOOKS in code; owner-made ones later share the id namespace) */
export interface CollageLook extends PieceStyle {
	id: string;
}

/**
 * How a character picture was picked (the editor re-resolves `src` from it); the renderer draws `src` only. Not
 * `character`: that key makes ChapterFiles save the whole persona offline and the Library add it to the cast.
 */
export interface PieceFigure {
	/** the character id */
	id: string;
	persona?: string;
	look?: string;
	mood?: string;
	view: CharacterImageVariant;
}

/** a score piece's place: a bar of the file – never a page, never a Verovio id */
export interface ScoreAt {
	/** the bar number as the file writes it (MusicXML measure@number = MEI measure@n): "12", "0", "6a" */
	bar: string;
	/** the n-th bar with that number; default 1 */
	nth?: number;
	/** the staff from the top (staff@n); default 1 – hidden in a condensed system: the bar's top staff */
	staff?: number;
	/** on the page that holds the bar (a background): `rect` in % of that page */
	page?: boolean;
}

/** one piece on a slice or a score (array order = paint order, back to front) */
export interface CollagePiece {
	/** `piece-<7 random>` */
	id: string;
	/** the file, in the host's path form: a toon document's `toonAssetRef`, a catalog row's CDN-root path */
	src: string;
	/** the file's width ÷ height, copied at pick time – laid out before load, never distorted */
	ratio: number;
	figure?: PieceFigure;
	/**
	 * x / y = the `anchor` point, the width; the height always follows `ratio` (never stored). Slice: % of the picture.
	 * Score (music): x = % of the bar's time (0 = its first beat, 100 = its end – it follows the notes, so a piece on a
	 * note stays on it whatever the line breaks), y = staff spaces below the staff's top line, width = staff spaces.
	 * Score (`at.page`): % of the page.
	 */
	rect: {
		x: number;
		y: number;
		width: number;
	};
	/** default 'tl'; score pieces are written 'c' */
	anchor?: Anchor;
	/** degrees around the centre */
	rotation?: number;
	/** a CollageLook id; none = as drawn */
	preset?: string;
	/** own values over the look */
	style?: PieceStyle;
	/** scores only (required there) */
	at?: ScoreAt;
	/** scores only: under the notes; default over */
	behind?: boolean;
}

/** a score's collage: `ScoreFiles.collage` (the full score) / `ScorePart.collage` (score.parts.<name>) */
export interface CollageSheet {
	/** the vault path it was placed on; another file on the part = "check the bars" */
	file: string;
	/** back → front within each side (over / behind) */
	pieces: CollagePiece[];
}

/** a piece's colouration as drawn: its look's values under its own, nulls (= off) dropped */
export type ResolvedPieceStyle = { [K in keyof PieceStyle]?: Exclude<PieceStyle[K], null> };

const INK = '#111111';
const RED = '#ff1900';
const CYAN = '#00d1ff';
const gradient = (from: string, to: string): PieceGradient => ({ stops: [{ at: 0, color: from }, { at: 100, color: to }] });

/** the code looks (from the lab Collage's inks, gradients and blends – COLLAGE.md §2 "Presets by reference") */
export const COLLAGE_LOOKS: readonly CollageLook[] = [
	{ id: 'ink', fill: INK },
	{ id: 'red', fill: RED },
	{ id: 'cyan', fill: CYAN },
	{ id: 'negative', fill: '#ffffff', blend: 'difference' },
	{ id: 'burn', fill: gradient(RED, '#b300ff'), blend: 'multiply' },
	{ id: 'acid', fill: gradient(CYAN, '#37ff00'), blend: 'exclusion' },
	{ id: 'sticker', outline: { color: '#ffffff', width: 3 }, shadow: { color: INK, x: 2, y: 2, blur: 0, opacity: 0.8 } },
	{ id: 'glow', shadow: { color: CYAN, x: 0, y: 0, blur: 6 } },
	{ id: 'ghost', opacity: 0.5, blur: { amount: 3 } },
	{ id: 'fade', blur: { amount: 6, from: { x: 50, y: 0 }, to: { x: 50, y: 100 } } }
];

export function collageLook(id: unknown): CollageLook | undefined {
	return typeof id === 'string' ? COLLAGE_LOOKS.find((look) => look.id === id) : undefined;
}

/** the keys a style has – nothing else is copied (a stored `__proto__` key would set the result's prototype) */
const STYLE_KEYS = ['blend', 'opacity', 'fill', 'outline', 'shadow', 'blur'] as const;

/** the piece's style as drawn: `{ ...look, ...piece.style }` – an own key replaces the look's whole, null = off */
export function pieceStyle(piece: Pick<CollagePiece, 'preset' | 'style'> | null | undefined): ResolvedPieceStyle {
	const look: PieceStyle = collageLook(piece?.preset) ?? {};
	const own: unknown = piece?.style;
	const merged: Record<string, unknown> = {};
	for (const key of STYLE_KEYS) {
		const value = isObj(own) && Object.hasOwn(own, key) && own[key] !== undefined ? own[key] : look[key];
		if (value !== undefined && value !== null) merged[key] = value;
	}
	return merged as ResolvedPieceStyle;
}

/** the drawn height in % of the box: `width` % of it, a file of `ratio` (w ÷ h), a box of `boxRatio` (its h ÷ w) */
export function pieceHeight(width: number, ratio: number, boxRatio: number): number {
	return width / (ratio * boxRatio);
}

/** what a style costs to draw (filters): outline 1, shadow 1, blur 1, a progressive blur 2 (two bands) */
export function heavyWeight(style: PieceStyle | null | undefined): number {
	if (!style) return 0;
	let weight = 0;
	if (style.outline && style.outline.width > 0) weight += 1;
	if (style.shadow) weight += 1;
	if (style.blur && style.blur.amount > 0) weight += style.blur.from && style.blur.to ? 2 : 1;
	return weight;
}

export type PieceKind = 'character' | 'shape' | 'image';

/** derived, never stored: a figure → a character picture, an .svg → a shape, else an image */
export function pieceKind(piece: Pick<CollagePiece, 'src' | 'figure'>): PieceKind {
	if (piece.figure) return 'character';
	return /\.svg$/i.test(piece.src ?? '') ? 'shape' : 'image';
}

/** a short name for lists: the character (· mood), else the file's name without folder and extension */
export function pieceName(piece: Pick<CollagePiece, 'src' | 'figure'>): string {
	const figure = piece.figure;
	if (figure?.id) return figure.mood ? `${figure.id} · ${figure.mood}` : figure.id;
	const file = String(piece.src ?? '').split('/').pop() ?? '';
	return file.replace(/(\.\dx)?\.[a-z0-9]+$/i, '') || '?';
}

/** warnings, not caps: how much a host should carry (sizes in % of the piece's width) */
export const COLLAGE_LIMITS = {
	slice: { pieces: 12, heavy: 4 },
	score: { pieces: 60, heavy: 20 },
	outline: 8,
	blur: 20,
	shadowBlur: 25
} as const;

/** the files a piece can draw (an <img> or a CSS mask) */
const PIECE_FILE = /\.(svg|webp|png|jpe?g|gif|avif)$/i;
const COLOR = /^#(?:[0-9a-f]{3}|[0-9a-f]{6})$/i;

/** a colour the renderer may write into CSS: #rgb | #rrggbb (PieceColor; the server's CatalogValidator HEX mirrors it) */
export function isPieceColor(v: unknown): v is string {
	return typeof v === 'string' && COLOR.test(v);
}

type Path = (string | number)[];
/** `path` is relative to the pieces list (`[]` = the list, `[i, 'rect']` = a piece's rect) */
export type CollageAdd = (level: 'error' | 'warning', path: Path, message: string) => void;

const isNum = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
const isObj = (v: unknown): v is Record<string, any> => !!v && typeof v === 'object' && !Array.isArray(v);
const whole = (v: unknown) => Number.isInteger(v) && (v as number) >= 1;
const unit = (v: unknown) => isNum(v) && v >= 0 && v <= 1;

/**
 * The problems of a host's pieces (slices: `checkSlice`; scores: `validateContainers`). Errors break the drawing;
 * warnings are what phones may struggle with (COLLAGE_LIMITS) and looks the code doesn't know.
 */
export function collageProblems(pieces: unknown, host: 'slice' | 'score', add: CollageAdd): void {
	if (pieces === undefined) return;
	if (!Array.isArray(pieces)) return add('error', [], 'pieces must be a list');
	const seen = new Set<string>();
	let heavy = 0;
	pieces.forEach((p: unknown, i: number) => {
		if (!isObj(p)) return add('error', [i], 'a piece must be an object');
		if (typeof p.id !== 'string' || !p.id) add('error', [i, 'id'], 'the piece has no id');
		else if (seen.has(p.id)) add('error', [i, 'id'], `piece id "${p.id}" is used twice`);
		else seen.add(p.id);
		if (typeof p.src !== 'string' || !PIECE_FILE.test(p.src.trim())) add('error', [i, 'src'], 'src must be a picture file (.svg, .webp, .png, .jpg, .gif, .avif)');
		if (!isNum(p.ratio) || p.ratio <= 0) add('error', [i, 'ratio'], "ratio must be the file's width ÷ height (> 0)");
		pieceRect(p.rect, [i, 'rect'], add);
		if (p.anchor !== undefined && !isAnchor(p.anchor)) add('error', [i, 'anchor'], `anchor must be one of ${ANCHORS.join(', ')}`);
		if (p.rotation !== undefined && !isNum(p.rotation)) add('error', [i, 'rotation'], 'rotation must be a number (degrees)');
		if (p.figure !== undefined) pieceFigure(p.figure, [i, 'figure'], add);
		if (p.preset !== undefined) {
			if (typeof p.preset !== 'string') add('error', [i, 'preset'], 'preset must be a look id');
			else if (!collageLook(p.preset)) add('warning', [i, 'preset'], `unknown look "${p.preset}" – the piece is drawn without it`);
		}
		if (p.style !== undefined) {
			if (!isObj(p.style)) add('error', [i, 'style'], 'style must be { blend?, opacity?, fill?, outline?, shadow?, blur? }');
			else styleProblems(p.style, [i, 'style'], add);
		}
		if (host === 'slice') {
			if (p.at !== undefined) add('error', [i, 'at'], 'at is for score pieces (a bar) – a slice piece is placed by rect alone');
			if (p.behind !== undefined) add('error', [i, 'behind'], 'behind is for score pieces – slice pieces always lie on the picture');
		} else {
			scoreAt(p.at, [i, 'at'], add);
			if (p.behind !== undefined && typeof p.behind !== 'boolean') add('error', [i, 'behind'], 'behind must be true or false');
		}
		heavy += heavyWeight(pieceStyle({ preset: p.preset, style: isObj(p.style) ? p.style : undefined }));
	});
	const limits = COLLAGE_LIMITS[host];
	if (pieces.length > limits.pieces) add('warning', [], `${pieces.length} pieces – more than ${limits.pieces} may be slow on phones`);
	if (heavy > limits.heavy) add('warning', [], `outlines, shadows and blurs weigh ${heavy} (more than ${limits.heavy}) – slow on phones`);
}

function pieceRect(rect: unknown, path: Path, add: CollageAdd) {
	if (!isObj(rect)) return add('error', path, 'rect { x, y, width } is missing');
	for (const key of ['x', 'y', 'width'] as const) if (!isNum(rect[key])) add('error', [...path, key], `rect.${key} must be a number`);
	if (isNum(rect.width) && rect.width <= 0) add('error', [...path, 'width'], 'rect.width must be > 0');
	if (rect.height !== undefined) add('error', [...path, 'height'], "a piece has no height – it follows the file's ratio (never distorted)");
}

function pieceFigure(figure: unknown, path: Path, add: CollageAdd) {
	if (!isObj(figure)) return add('error', path, 'figure must be { id, persona?, look?, mood?, view }');
	if (typeof figure.id !== 'string' || !figure.id) add('error', [...path, 'id'], 'figure.id must be a character id');
	for (const key of ['persona', 'look', 'mood'] as const) {
		if (figure[key] !== undefined && typeof figure[key] !== 'string') add('error', [...path, key], `figure.${key} must be a string`);
	}
	if (!(CHARACTER_IMAGE_VARIANTS as readonly unknown[]).includes(figure.view)) add('error', [...path, 'view'], `figure.view must be one of ${CHARACTER_IMAGE_VARIANTS.join(', ')}`);
}

function scoreAt(at: unknown, path: Path, add: CollageAdd) {
	if (!isObj(at)) return add('error', path, 'a score piece needs at { bar } (the bar it sticks to)');
	if (typeof at.bar !== 'string' || !at.bar.trim()) add('error', [...path, 'bar'], 'at.bar must be the bar number as the file writes it ("12", "6a")');
	if (at.nth !== undefined && !whole(at.nth)) add('error', [...path, 'nth'], 'at.nth must be a whole number ≥ 1');
	if (at.staff !== undefined && !whole(at.staff)) add('error', [...path, 'staff'], 'at.staff must be a whole number ≥ 1 (from the top)');
	if (at.page !== undefined && typeof at.page !== 'boolean') add('error', [...path, 'page'], 'at.page must be true or false');
}

const color = (value: unknown, path: Path, add: CollageAdd) => {
	if (!isPieceColor(value)) add('error', path, 'a colour is #rgb or #rrggbb');
};
const point = (value: unknown, path: Path, add: CollageAdd) => {
	if (!isObj(value) || !isNum(value.x) || !isNum(value.y)) add('error', path, 'a point is { x, y } in % of the piece');
};

/** a piece's own style (the code looks are known to be right) */
function styleProblems(style: Record<string, any>, path: Path, add: CollageAdd) {
	if (style.blend !== undefined && !(BLEND_MODES as readonly unknown[]).includes(style.blend)) add('error', [...path, 'blend'], `blend must be one of ${BLEND_MODES.join(', ')}`);
	if (style.opacity !== undefined && !unit(style.opacity)) add('error', [...path, 'opacity'], 'opacity must be 0…1');
	const fill = style.fill;
	if (typeof fill === 'string') color(fill, [...path, 'fill'], add);
	else if (isObj(fill)) gradientProblems(fill, [...path, 'fill'], add);
	else if (fill !== undefined && fill !== null) add('error', [...path, 'fill'], 'fill must be a colour, a gradient { angle?, stops } or null');
	const outline = style.outline;
	if (isObj(outline)) {
		color(outline.color, [...path, 'outline', 'color'], add);
		if (!isNum(outline.width) || outline.width < 0) add('error', [...path, 'outline', 'width'], 'outline.width must be a number ≥ 0 (% of the piece)');
		else if (outline.width > COLLAGE_LIMITS.outline) add('warning', [...path, 'outline', 'width'], `an outline over ${COLLAGE_LIMITS.outline} % of the piece gets boxy corners`);
	} else if (outline !== undefined && outline !== null) add('error', [...path, 'outline'], 'outline must be { color, width } or null');
	const shadow = style.shadow;
	if (isObj(shadow)) {
		color(shadow.color, [...path, 'shadow', 'color'], add);
		for (const key of ['x', 'y'] as const) if (!isNum(shadow[key])) add('error', [...path, 'shadow', key], `shadow.${key} must be a number (% of the piece)`);
		if (!isNum(shadow.blur) || shadow.blur < 0) add('error', [...path, 'shadow', 'blur'], 'shadow.blur must be a number ≥ 0 (% of the piece)');
		else if (shadow.blur > COLLAGE_LIMITS.shadowBlur) add('warning', [...path, 'shadow', 'blur'], `a shadow blur over ${COLLAGE_LIMITS.shadowBlur} % of the piece is slow on phones`);
		if (shadow.opacity !== undefined && !unit(shadow.opacity)) add('error', [...path, 'shadow', 'opacity'], 'shadow.opacity must be 0…1');
	} else if (shadow !== undefined && shadow !== null) add('error', [...path, 'shadow'], 'shadow must be { color, x, y, blur, opacity? } or null');
	const blur = style.blur;
	if (isObj(blur)) {
		if (!isNum(blur.amount) || blur.amount < 0) add('error', [...path, 'blur', 'amount'], 'blur.amount must be a number ≥ 0 (% of the piece)');
		else if (blur.amount > COLLAGE_LIMITS.blur) add('warning', [...path, 'blur', 'amount'], `a blur over ${COLLAGE_LIMITS.blur} % of the piece is slow on phones`);
		if ((blur.from === undefined) !== (blur.to === undefined)) add('error', [...path, 'blur'], 'a ramp needs both from and to (or neither: an even blur)');
		if (blur.from !== undefined) point(blur.from, [...path, 'blur', 'from'], add);
		if (blur.to !== undefined) point(blur.to, [...path, 'blur', 'to'], add);
	} else if (blur !== undefined && blur !== null) add('error', [...path, 'blur'], 'blur must be { amount, from?, to? } or null');
}

function gradientProblems(fill: Record<string, any>, path: Path, add: CollageAdd) {
	if (fill.angle !== undefined && !isNum(fill.angle)) add('error', [...path, 'angle'], 'angle must be a number (CSS degrees)');
	if (!Array.isArray(fill.stops) || fill.stops.length < 2) return add('error', [...path, 'stops'], 'a gradient needs at least 2 stops');
	let last = -Infinity;
	fill.stops.forEach((stop: unknown, i: number) => {
		const at = [...path, 'stops', i];
		if (!isObj(stop)) return add('error', at, 'a stop is { at, color, opacity? }');
		if (!isNum(stop.at) || stop.at < 0 || stop.at > 100) add('error', [...at, 'at'], 'stop.at must be 0…100 (% along the line)');
		else if (stop.at < last) add('error', [...at, 'at'], 'stops must be in ascending order');
		else last = stop.at;
		color(stop.color, [...at, 'color'], add);
		if (stop.opacity !== undefined && !unit(stop.opacity)) add('error', [...at, 'opacity'], 'stop.opacity must be 0…1');
	});
}
