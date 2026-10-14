// The home view's input (agents/research/CAVE.md): resolved URLs and names – the app owns the data (catalog, inventory,
// API) and the language; the view only draws and reports what the reader does. No app state (PLATFORM.md dogmas).

/** Something the reader carries (an inventory entry): an item, or a piece of furniture not placed yet */
export interface HomeThing {
	/** the inventory entry id */
	id: string;
	name: string;
	/** the item icon's URL */
	icon: string;
	/** a furniture item – tapping the room places it */
	furniture?: boolean;
}

/** A piece of furniture standing in the home */
export interface HomePiece {
	/** the inventory entry id */
	id: string;
	name: string;
	/** its home image's URL (the icon while it has none) */
	image: string;
	/** its width in % of the room's front (homes.ts `ItemFurniture.size`) */
	size: number;
	/** its foot, % of the room */
	x: number;
	y: number;
	/** mirrored – faces left (for the right wall) */
	flip?: boolean;
	/** a workbench: the workshop recipes run here */
	workshop?: boolean;
	/** a rack: how many fit (null = no limit); none = holds nothing */
	holds?: { max: number | null } | null;
	/** what lies in it */
	contents: HomeThing[];
}

/** A workshop recipe at the workbench */
export interface HomeRecipe {
	id: string;
	name: string;
	/** the reader has the inputs */
	possible: boolean;
}

/** The home scene's files at the reader's media quality (URLs) */
export interface HomeSceneFiles {
	/** the still (also the video's poster); '' = none yet – a plain drawn room */
	image: string;
	/** a short muted loop */
	video?: string | null;
}

/** the view's texts (the app's language); English by default */
export interface HomeLabels {
	leave: string;
	visitLeave: string;
	inventory: string;
	visiting: string;
	/** "{count} / {max}" */
	count: string;
	putInHint: string;
	placeHint: string;
	turn: string;
	workshop: string;
	noRecipes: string;
	/** "{name} – drag to move" */
	dragHint: string;
	room: string;
}

export const DEFAULT_HOME_LABELS: HomeLabels = {
	leave: 'Leave home',
	visitLeave: 'Leave the visit',
	inventory: 'Your inventory',
	visiting: 'Your inventory (you are visiting)',
	count: '{count} / {max}',
	putInHint: 'Select something in your inventory, then tap the {name}.',
	placeHint: 'Tap a place in the room – the furniture stays in this home from then on.',
	turn: 'Turn',
	workshop: 'Workshop',
	noRecipes: 'Nothing to make here right now.',
	dragHint: '{name} – drag to move',
	room: 'The room'
};

export const fill = (text: string, values: Record<string, string | number>): string =>
	text.replace(/\{(\w+)\}/g, (m, k) => (k in values ? String(values[k]) : m));
