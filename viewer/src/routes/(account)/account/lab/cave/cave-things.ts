// Draft data of the cave (research/CAVE.md D4): placeholders for the furniture and media items to come.

export type Kind = 'cd' | 'tape' | 'vinyl' | 'book' | 'broken_book' | 'furniture';
export type Thing = { uid: number; item: string; name: string; kind: Kind };
export type Furniture = Thing & {
	/** its size in the home: the width in % of the room's width at the front (proposal: ItemDef.furniture.size) */
	size: number;
	holds?: { kinds: Kind[]; max: number; label: string };
	workshop?: boolean;
};
/** a piece of furniture in the room: where it stands (% of the room) and what lies in it */
export type Placed = { f: Furniture; x: number; y: number; contents: Thing[] };

let uid = 100;
export const thing = (item: string, name: string, kind: Kind): Thing => ({
	uid: uid++,
	item,
	name,
	kind
});

const FURNITURE: Record<string, Omit<Furniture, 'uid'>> = {
	cd_rack: {
		item: 'cd_rack',
		name: 'CD rack',
		kind: 'furniture',
		size: 12,
		holds: { kinds: ['cd'], max: 50, label: 'CDs' }
	},
	cassette_rack: {
		item: 'cassette_rack',
		name: 'Cassette rack',
		kind: 'furniture',
		size: 10,
		holds: { kinds: ['tape'], max: 50, label: 'cassettes' }
	},
	record_shelf: {
		item: 'record_shelf',
		name: 'Record shelf',
		kind: 'furniture',
		size: 22,
		holds: { kinds: ['vinyl'], max: 50, label: 'records' }
	},
	workbench: { item: 'workbench', name: 'Workbench', kind: 'furniture', size: 32, workshop: true },
	sofa: { item: 'sofa', name: 'Sofa', kind: 'furniture', size: 40 }
};
/** depth: further back (higher up in the room) = smaller – 65 % at the back wall's foot, 100 % at the front edge */
export const depth = (y: number) => 0.65 + 0.35 * Math.min(1, Math.max(0, (y - 55) / 45));
export const furn = (id: string): Furniture => ({ uid: uid++, ...FURNITURE[id] });
