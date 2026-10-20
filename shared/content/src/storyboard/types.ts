// The storyboard's document types (#119): the scene file format `"storyboard": 2` (workflows/scenes/storyboard.md "A
// shot", "The storyboard file") and the per-chapter document the server stores as `storyboard:<toon>:<chapter>` with
// its scenes inline (agents/research/STUDIO-IN-CREATOR.md §2.2). Derived from the 14 scene files of chapters 3–5 and
// the studio / renderer's fields. Unknown words in the vocab fields are what validate() reports – the types name the
// allowed ones.

import type { DialogTail } from '../content.ts';
import type {
	Facing, LightHeight, LightKey, LightShadow, LightSource, Move, Pose, Projection, ReviewStatus, ShotAngle, ShotFormat,
	ShotSize, ShotStatus, SourceKind, View
} from './vocab.ts';

/** a text in locales: `{ de: '…' }` (German first; other locales come with the translation step) */
export type LocaleText = Record<string, string>;

/** a cast entry: one persona of a registry character (`cast:<toon>`), or a figure the registry lacks (`new`) */
export interface CastEntry {
	ref: string;
	/** registry character id; null = not in the registry yet (`new` says who – it goes to the Wanted list) */
	character: string | null;
	persona?: string;
	look?: string;
	new?: string;
}

/** a place of the scene: MAP.md's place id (or null + `new`) and the time / weather */
export interface LocationEntry {
	ref: string;
	place?: string | null;
	time?: string;
	new?: string;
}

/** the one accent colour allowed in the frame, and on what */
export interface LightAccent {
	color: string;
	on?: string;
}

/** the light (workflow "light"): one extremely hard key; a scene default at the top, a shot lists what differs */
export interface Light {
	key?: LightKey;
	height?: LightHeight;
	source?: LightSource;
	shadow?: LightShadow;
	rim?: boolean;
	/** 0 (pure Sin City) … 1 (soft), default 0.5 */
	soft?: number;
	/** only 'hard' is valid (validate errors on any other) */
	quality?: 'hard';
	/** the 3D view's free key direction (lumaPrompt): degrees around the subject / up */
	azimuth?: number;
	elevation?: number;
	accent?: LightAccent;
	note?: LocaleText;
}

/**
 * The shot's camera from the 3D view (tools/storyboard-render core.js): a free camera `{ pos, target, fov, roll }`
 * (metres, Y up) or adjustments of the preset camera `{ dist, height, pitch, yaw, roll, fov }`.
 */
export interface Camera {
	pos?: [number, number, number];
	target?: [number, number, number];
	fov?: number;
	roll?: number;
	dist?: number;
	height?: number;
	pitch?: number;
	yaw?: number;
}

/** a figure in a shot: a cast ref, where (x across, y along the shot, 0…1) and how big (h = fraction of a screen) */
export interface Figure {
	cast: string;
	framing?: ShotSize;
	mood?: string;
	pose?: Pose;
	facing?: Facing;
	x?: number;
	y?: number;
	h?: number;
	/** overrides of the cast entry's persona / look */
	persona?: string;
	look?: string;
	/** extra turn in degrees (the 3D view) */
	turn?: number;
	/** the figure's depth in the 3D view */
	z?: number;
	scale?: number;
	/** false keeps it in the air where drawn (a hand camera would otherwise put it on the ground) */
	ground?: boolean;
	note?: string;
}

/** a text box of a shot: content.ts Dialog's look fields, text as a locale map, rect in % of the shot */
export interface SbDialog {
	variant?: 'narration' | 'bubble' | 'caption' | 'lyrics';
	font?: 'mono' | 'lettering';
	tail?: DialogTail;
	rect: { x: number; y: number; width?: number; height?: number };
	text: LocaleText;
	/** a scene-heading box */
	title?: boolean;
	/** font size in % of the width */
	size?: number;
}

export interface ShotSource {
	/** where the image comes from now (schematic until matched) */
	type?: SourceKind;
	/** what it should come from */
	want?: SourceKind;
	/** the Figma node id once matched */
	figma?: string;
}

/** the owner's verdict (approve.mjs); an open `change` is a warning until it is done */
export interface Review {
	status: ReviewStatus;
	note?: string;
	date?: string;
}

/** one handheld sample: t (s), position (m, Y up), quaternion [x, y, z, w], fov (°) */
export interface TakeSample {
	t: number;
	position: [number, number, number];
	quaternion: [number, number, number, number];
	fov?: number;
}

/** a recorded handheld camera take (research/HANDHELD-CAMERA.md) */
export interface Take {
	/** t1, t2 … */
	id: string;
	created: string;
	smooth: number;
	scale: number;
	in: number;
	out: number;
	samples: TakeSample[];
}

/** a shot's takes (the studio's takes/<shot>.json): the handheld camera on, the active take */
export interface ShotTakes {
	on: boolean;
	active: string | null;
	takes: Take[];
}

/** one shot: one image used for layouting the toon (owner 2026-10-09) */
export interface Shot {
	/** sb-<scene>-<n>, never reused */
	id: string;
	format: ShotFormat;
	/** w:h ("9:16", "1:4"; the panorama's w:h for loop360) */
	aspect: string;
	projection?: Projection;
	move?: Move;
	shot?: { size?: ShotSize; angle?: ShotAngle };
	location?: string;
	view?: View;
	light?: Light;
	camera?: Camera;
	action: LocaleText;
	/** [[from %, to %]] along the shot: zones with nothing to read */
	air?: [number, number][];
	figures?: Figure[];
	dialog?: SbDialog[];
	source?: ShotSource;
	/** only for shots that want `generated`: the image in English */
	prompt?: { en?: string };
	review?: Review;
	status?: ShotStatus;
	note?: string;
	/** a scene-heading shot */
	title?: boolean;
	/** the handheld camera's takes (the online document holds them inline, §2.2) */
	takes?: ShotTakes;
	/** the chapter's own types where the script asks (SliceItem, VNEvent, audio) – kept as they are */
	items?: unknown[];
	events?: unknown[];
	audio?: unknown;
}

/** a VN event of a storyboard group (measureVn reads showMessage / showChoices / joinScene / leaveScene) */
export interface SbEvent {
	type: string;
	params?: { text?: LocaleText | string; [k: string]: unknown };
	[k: string]: unknown;
}

/** a group of shots: one vertical group per scene by default; horizontal + endless for a loop360 */
export interface StoryboardGroup {
	label: string;
	renderer?: { type: 'vertical' | 'horizontal' | string; params?: { endless?: boolean; [k: string]: unknown } };
	continues?: boolean;
	shots?: Shot[];
	events?: SbEvent[];
}

/** one script scene's storyboard (the scene file, format version 2) */
export interface StoryboardDoc {
	storyboard: 2;
	toon: string;
	chapter?: number;
	source: { script?: string; scene: number; heading?: string; lines?: number[] };
	cast?: CastEntry[];
	locations?: LocationEntry[];
	/** the scene's default light */
	light?: Light;
	groups: StoryboardGroup[];
	/** decisions / questions for the owner */
	notes?: string[];
}

/** the per-chapter document `storyboard:<toon>:<chapter>`: the chapter's scenes inline, in scene order */
export interface ChapterStoryboard {
	storyboard: 2;
	toon: string;
	chapter: number;
	scenes: StoryboardDoc[];
}
