// GENERATED from server/schema/content/scene.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
// The stored scene document `scene:<id>` (#243, research/SCENES.md §3): deck-space's SceneDocument (deck/space/src/scene_document.rs) →
// its JSON Schema (deck-schema) → these types. Whether a scene plays is the core's (the build service's /validate at publish).

/** the format the API stores and the decks read */
export const SCENE_FORMAT = 2 as const;

export type Act = 'join' | 'leave';

/** the part of the view a layer covers */
export type Anchor = 'full' | 'left' | 'right' | 'top' | 'bottom';

/** A spawn zone's arrangement: its kind and the spacing between members. */
export type Arrange = {
	kind: ArrangeKind;
	/** metres between neighbours (a set's spot is a point: the spacing says how wide the zone grows) */
	spacing?: number;
};

/** The arrangements of a first version (scene_resolve.rs `arrange`): more (a stack, a crowd) when a use needs one. */
export type ArrangeKind = 'row' | 'cluster';

/** A module attached to a slot of an entity, on that entity's document. */
export type Attach = {
	/** the module's id (`dolly`): the catalog's `module:<id>` */
	module: string;
	/** its parameters by name; the rest at their defaults */
	params?: Record<string, number>;
	/** the slot's domain word on this entity (`motion`) */
	slot: string;
};

/** One of a colour, an image or a video. */
export type Background = {
	/** `#rgb` or `#rrggbb` */
	colour?: string | null;
	/** a picture's media path, laid over the stage */
	image?: string | null;
	/** a video's media path, laid over the stage */
	video?: string | null;
};

export type Ease = 'linear' | 'in' | 'out' | 'in_out' | 'whip';

/** `start` or `join`. */
export type Enter = 'start' | 'join';

/** The filter of a placeholder (SCENES.md §9.2): the address book's typeahead question, stored. Within a fact any value admits; every fact given must hold (owner 2026-11-12, §9.7 Q5). */
export type Fill = {
	/** the address book's kinds that may fill it (required): a holder's of `character`, `persona`, `look`, `item`; a location's `location` */
	kind: ('character' | 'persona' | 'look' | 'item' | 'location')[];
	/** only entries below this reference (`character:catpope`: her personas and looks) */
	parent?: string | null;
	/** a hand-picked list of references (`character:catpope`, `item:incense`); none = any of the kinds */
	ref?: string[];
};

/** One filler or several (a spawn zone's members). */
export type Fillers = string | string[];

/** One place on the stage (SCENE-PACKAGE §2, SCENES.md §9.3). */
export type Holder = {
	/** a spawn zone's arrangement of its members within the spot (required with `take: many`) */
	arrange?: Arrange | null;
	/** where it stands from its spot, metres (x across, y up, z deep): a member moved on the stage */
	at?: [number, number, number] | null;
	/** `start` (the default): on stage when the scene starts; `join`: waits for the story (`joinScene`, SCENES.md §9.4) */
	enter?: Enter;
	/** a placeholder's filter – or `ref` */
	fill?: Fill | null;
	/** its name in the document and in a use's `with` (`left`, `who`); not an address – a filled holder's address is its filler's place (`scene:stage/character:catpope/…`), an empty one has none */
	id: string;
	/** a placeholder's name for people */
	label?: Record<string, string> | null;
	/** a spawn zone's limit; none = any number */
	max?: number | null;
	/** the modules on its member's slots (deck-catalog `slot`: an item's `motion`, on top of its spot) */
	module?: Attach[];
	/** fillers for the editor and the browser (a spawn zone may preview several) */
	preview?: Fillers | null;
	/** an authored filler (`character:catpope`, `item:incense`) – or `fill`, never both */
	ref?: string | null;
	/** the rig its filler plays; none = the filler's own (`character:catpope` → `rig:character.catpope`) */
	rig?: RigPin | null;
	/** the spot of the set where it stands and enters (`spot_left`); none = the scene's spot */
	spot?: string | null;
	/** `one` filler (the default) or `many`: a spawn zone (owner 2026-11-12 [scenes]) – a placeholder only */
	take?: Take;
	/** values and preset choices only (SCENE-PACKAGE §2), by address: `"voice:voice.index": 4.0`, `"voice:@preset": "cathedral_voice.bright"` */
	values?: Record<string, HolderValue>;
};

/** A holder's value: a parameter's number, or a track's preset choice. */
export type HolderValue = number | string;

export type Layer = {
	/** the part of the view it covers: all of it, or a strip at an edge */
	anchor?: Anchor;
	/** metres behind the camera's pivot; below 0 in front of it */
	depth: number;
	/** the picture's media path (a PNG with transparency, or a video – `background::is_video`); none = an empty place */
	image?: string | null;
};

export type Length = {
	beats: number;
} | {
	bars: number;
} | {
	seconds: number;
};

export type Lens = {
	/** mm on a full-frame sensor (perspective) */
	focal?: number;
	/** metres from the eye to where it is sharp; none = at the aim */
	focus?: number | null;
	projection?: Projection;
	/** the view's height in metres (orthographic) */
	size?: number;
};

export type LinkDef = {
	from: string;
	/** an audio link's channel map (plan 2.6, #318): `[source, target]` pairs counted from 1 – `[[1, 3], [2, 4]]` lands a stereo source on channels 3 and 4 (a send's offset); one source may go to several targets, several sources into one target sum. Empty = channel n to channel n. `scale` on an audio link is the send level. */
	map?: [number, number][];
	offset?: number;
	scale?: number;
	to: string;
};

/** event of type `on` (the chapter's `VNEvent` types, protocol::story_events), with this `id` when given, naming a character this `character` admits when given (itself, or a persona / look below it), makes each such character `do` its word. */
export type Listener = {
	/** only the characters this reference admits (`character:catpope`: her and every persona of hers) */
	character?: string | null;
	/** what each admitted character of the event does on this stage */
	do: Act;
	/** only the story event with this id (the creator's stable event id) */
	id?: string | null;
	/** the story event's type (`joinScene`, `leaveScene`, `showMessage` …) */
	on: string;
};

/** A location no scene names yet: what may fill it. */
export type LocationPlaceholder = {
	/** the filter: `kind` is `["location"]` */
	fill: Fill;
	/** its name in a use's `with` (`where`, `home`); not an address */
	id: string;
	/** its name for people */
	label?: Record<string, string> | null;
	/** a filler for the editor and the browser */
	preview?: string | null;
};

/** The scene's location: a reference, or a placeholder. */
export type LocationSlot = string | LocationPlaceholder;

/** A named move of the scene: `scene:<id> > move:<id>` plays it. */
export type MoveDef = {
	/** shake: degrees of unrest */
	amount?: number;
	ease?: Ease;
	/** follow: what it keeps in the shot, as a shot's `aim` names it */
	follow?: string | null;
	/** crane: metres above (or below) the higher (lower) end */
	height?: number;
	id: string;
	kind: MoveKind;
	length: Length;
	repeat?: Repeat;
	/** orbit, spiral */
	rounds?: number;
	/** in */
	side?: Side;
	start?: Start;
	/** the move that follows when it ends (a chain) */
	then?: string | null;
	/** the shot it ends on */
	to: string;
	turn?: Turn;
};

export type MoveKind = 'cut' | 'glide' | 'in' | 'fly' | 'crane_down' | 'crane_up' | 'spiral' | 'push' | 'pull' | 'orbit' | 'truck' | 'tilt' | 'zoom' | 'dolly_zoom' | 'whip' | 'follow' | 'shake' | 'rack';

export type NodeDef = {
	id: string;
	kind: string;
	[key: string]: unknown;
};

/** Who may take a camera on a phone (LIVE-CAMERAS.md §4, §5): the one condition control's filter (any one fact; none: anyone with the code) and the time without a pose before it returns (none: only Leave, a take-over or the conductor). */
export type Phones = {
	for?: Record<string, unknown> | null;
	release_ms?: number | null;
};

export type Projection = 'perspective' | 'orthographic';

/** What a move does once it arrived: stays, runs again from its start, or runs back and forth – until another move, a cut, a hand on the view or the row's end. A move in beats or bars repeats on the bar line. */
export type Repeat = 'once' | 'loop' | 'ping_pong';

/** A rig by reference and the version it was wired against (SCENE-PACKAGE §2). */
export type RigPin = {
	/** `rig:<subject>` (`rig:character.catpope`) */
	ref: string;
	/** the version it was wired against: the rig's `version` as scene-content serves it (16 hex of its data's SHA-256 – the catalog keeps no version number); none = the current one */
	version?: string | null;
};

/** The scene's primary camera. */
export type SceneCamera = {
	/** the eye height (m above the floor) of the primary camera while the scene has no shots: head-on at the stage's centre, level */
	height?: number;
	/** the lens before a shot is taken */
	lens?: Lens;
	/** the modules on its slots (deck-catalog `slot`: `motion`, on top of its shots and moves) */
	module?: Attach[];
	move?: MoveDef[];
	/** a phone in a live session may take the primary camera (`scene:<id>`) */
	phones?: Phones | null;
	/** its shots; the first opens the scene */
	shot?: ShotDef[];
	source?: Source;
	track?: Track | null;
};

/** What a switch does to the outputs (the lamps) between two scenes (#264). */
export type SceneTransition = 'fade' | 'cut' | 'blackout';

/** A use of a scene (SCENES.md §9.2, #504): a scene plus the fillers for its placeholders – what a group, a cell, a location or a character stores instead of a new scene document when only the fillers change (owner 2026-11-12 [scenes], §9.7 Q3). Whether it plays is the core's: `Use::problems` (how the server would get this verdict is the owner's review, #506). */
export type SceneUse = {
	/** the scene it plays: `scene:<id>` */
	scene: string;
	/** its fillers by placeholder id (the location placeholder's or a holder's `id`): a reference, or several for a spawn zone; a placeholder left out stays empty, or is filled by context (the receiver's) or the story (`joinScene`) */
	with?: Record<string, Fillers>;
};

export type ShotDef = {
	/** what it aims at by address in place of `target` (rig/address.rs `Aim`): a holder, `character:<id>`, `item:<id>`, `spot:<name>` – the shot keeps its angles and distance around it, wherever it stands */
	aim?: string | null;
	/** metres from the aim */
	distance: number;
	/** what it keeps in focus by address, as `aim` names it: the focus follows it (its distance from the eye each frame) in place of the lens's `focus` */
	focus_on?: string | null;
	/** a word: `scene:<id> > shot:<id>` picks it */
	id: string;
	label?: Record<string, string>;
	lens?: Lens;
	/** a phone in a live session may take this shot as a camera of its own (`scene:<id> > shot:<id>`) */
	phones?: Phones | null;
	/** degrees above the aim */
	pitch: number;
	/** where it aims (m, the set's space) */
	target: [number, number, number];
	/** degrees around the aim: 0 looks along −z from +z */
	yaw: number;
};

export type Side = 'left' | 'right';

/** What steers the camera. */
export type Source = 'scene' | 'handheld';

export type StageSettings = {
	/** the audio device's buffer in frames (#208, deck-native's audio/backend.rs): a heavy scene may ask for more; `--audio-buffer` and deck.toml's `audio_buffer` (the machine's) win over it. None = the device's default */
	audio_buffer?: number | null;
	/** the stage's background; a played scene carries the one that won (`deck_catalog::background::resolve`) */
	background?: Background | null;
	/** the 2D layers around the 3D, drawn far to near; a front one over it */
	layers?: Layer[];
	name?: string;
	/** the render size relative to the window (0.25–1); a software adapter halves it by itself */
	render_scale?: number;
	/** how the stage hands over TO this scene on a switch (F5, #264): the DMX / MIDI outputs of the scene that was playing fade into this one's (`fade`, the default), snap (`cut`) or go dark first (`blackout`); a stop is always dark */
	transition?: SceneTransition;
	/** the hand-over's length in s (`fade`); the same time base as the picture's fade in (deck-native switcher.rs `FADE_IN`) */
	transition_secs?: number;
};

/** When a move starts. */
export type Start = 'now' | 'bar';

/** How many fillers a holder takes. */
export type Take = 'one' | 'many';

/** The trolley track: its first point is the start, its last the end. */
export type Track = {
	point: TrackPoint[];
};

/** A point of the trolley track: where it is, its Bézier handle (the way out of it; the way in is its mirror), and when a fly-through reaches it. */
export type TrackPoint = {
	at: [number, number, number];
	/** the ease of the way here from the point before; none = an even speed */
	ease?: Ease | null;
	/** metres from `at`; none = a smooth curve through the neighbours */
	handle?: [number, number, number] | null;
	/** when the fly-through arrives here, counted from its start (beats, bars or seconds); none = by the track's length between the points that have one (the end without one: the move's length). The start has none. */
	time?: Length | null;
};

export type Turn = 'right' | 'left';

/** One `scene:<id>` document as the API stores it. */
export type SceneDocument = {
	/** its primary camera: shots, track, moves (scene_camera.rs) */
	camera?: SceneCamera | null;
	/** the document's format: 2 (location, holders, spawn zones; #489) */
	format: 2;
	/** the places on its stage, in the order a story's `joinScene` tries them (SCENES.md §9.4) */
	holder?: Holder[];
	/** the key's id: `nave` for `scene:nave` – platform-wide, no toon in it (a scene can serve several toons and shows) */
	id: string;
	link?: LinkDef[];
	/** optionally its id and a character → join | leave (scene_play.rs `Playing::story`) */
	listener?: Listener[];
	/** where it plays: `location:<id>`, or a placeholder a use fills; none = the `space` node's own `set` (or no set) */
	location?: LocationSlot | null;
	/** the scene's own nodes (the space, mixers); the holders' rigs are flattened in by the resolve step */
	node?: NodeDef[];
	/** the parameters as outputs (#211, scene.rs `Scene::outputs`) */
	outputs?: Record<string, unknown> | null;
	stage?: StageSettings;
	/** the toons whose address books list it; empty = a template, listed in every toon's book (#504, `is_template`) */
	toons?: string[];
	/** the number the server stamps on every change (as a show's); absent until the first save */
	version?: number | null;
};
