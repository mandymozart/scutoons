// Draft data of the cave (research/CAVE.md D4): placeholders for the furniture and media items to come.

export type Kind = 'cd' | 'tape' | 'vinyl' | 'book' | 'broken_book' | 'furniture';
export type Thing = { uid: number; item: string; name: string; kind: Kind };
export type Furniture = Thing & {
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
		holds: { kinds: ['cd'], max: 12, label: 'CDs' }
	},
	cassette_rack: {
		item: 'cassette_rack',
		name: 'Cassette rack',
		kind: 'furniture',
		holds: { kinds: ['tape'], max: 12, label: 'cassettes' }
	},
	record_shelf: {
		item: 'record_shelf',
		name: 'Record shelf',
		kind: 'furniture',
		holds: { kinds: ['vinyl'], max: 20, label: 'records' }
	},
	workbench: { item: 'workbench', name: 'Workbench', kind: 'furniture', workshop: true }
};
export const furn = (id: string): Furniture => ({ uid: uid++, ...FURNITURE[id] });
