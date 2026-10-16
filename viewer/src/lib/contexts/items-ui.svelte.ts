import { setContext, getContext } from 'svelte';
import { legacyPlayAudio, type Entry, type ModelSource, type ScanObjectSkill, type Skill } from '@scutoons/content';

const ITEMS_UI_KEY = Symbol('items-ui');

export interface CameraRequest {
	/** QR scanner (ignored when `object` is set) */
	qr: boolean;
	/** object detection (the cucumber camera): the session only detects, no QR decoding */
	object: ScanObjectSkill | null;
	/** entry whose skill opened the camera (for the header) */
	entry: Entry | null;
}

/** What the audio player plays: an album's tracks in order (one track for the old `play_audio` skill) */
export interface AudioRequest {
	/** `id` names the song's credits section (`songSection`); `duration` (s) fills the seek bar until the file says;
	 *  `side` = the record side (none = A) */
	tracks: { id?: string; src: string; title: string; duration?: number; side?: 'A' | 'B' }[];
	/** the album's title in the reader's language */
	title: string;
	/** the toon whose credits (`credits:<toon>`) name the songs' people – the expanded card shows them */
	toon?: string | null;
	/** the media item it plays from (its icon in the player), if any */
	entry: Entry | null;
	/** the album's cover (CDN-root path) – the record's label in the player */
	cover?: string | null;
	/** the playing item's 3D model (a CDN-root `.glb` or the shared record model's look, ITEMS.md "3D item models") –
	 *  the card's 3D record player */
	model?: ModelSource | null;
	/** track `i`'s playable address at the reader's media quality, asked for when it starts (a vault track's signed URL
	 *  lives ten minutes – asked again when it ran out); without it `tracks[i].src` plays as it is */
	sign?: (i: number) => Promise<string>;
}

export type ItemsUiContext = ReturnType<typeof createItemsUiContext>;

/**
 * Global overlays driven by item skills and codes, rendered by components/items/items-host.svelte
 * in the root layout: camera, claim sheet, minigame, audio player.
 */
export function createItemsUiContext() {
	let camera = $state<CameraRequest | null>(null);
	let claimCode = $state<string | null>(null);
	let claimAuto = $state(false);
	let claimDispute = $state(false);
	let minigame = $state<{ id: string; entry: Entry | null } | null>(null);
	let audio = $state<AudioRequest | null>(null);
	let inventoryRequested = $state(false);

	/**
	 * Run the "use" skill of an item. A camera session has one mode: an item with `scan_object` (the cucumber camera)
	 * opens straight in object detection and never decodes QR codes; `scan_qr` (the starter camera) opens the QR scanner.
	 */
	function use(entry: Entry | null, skills: Skill[]) {
		const object = (skills.find((s) => s.type === 'scan_object') as ScanObjectSkill | undefined) ?? null;
		if (object) {
			camera = { qr: false, object, entry };
			return;
		}
		if (skills.some((s) => s.type === 'scan_qr')) {
			camera = { qr: true, object: null, entry };
			return;
		}
		const game = skills.find((s) => s.type === 'minigame');
		if (game && game.type === 'minigame') {
			minigame = { id: game.id, entry };
			return;
		}
		// the old `play_audio` skill (before albums + players, 2026-10-02): plays without a player
		const src = legacyPlayAudio(skills);
		if (src) audio = { tracks: [{ src, title: '' }], title: '', entry };
	}

	return {
		get camera() {
			return camera;
		},
		get claimCode() {
			return claimCode;
		},
		get claimAuto() {
			return claimAuto;
		},
		/** open the claim sheet straight in "This is mine" (ownership claim; the QR landing page's `?claim=1`) */
		get claimDispute() {
			return claimDispute;
		},
		get minigame() {
			return minigame;
		},
		get audio() {
			return audio;
		},
		/** set when something (e.g. `?inventory=open`, a claim) wants the toon inventory overlay open */
		get inventoryRequested() {
			return inventoryRequested;
		},
		use,
		openCamera: (request: CameraRequest) => (camera = request),
		closeCamera: () => (camera = null),
		openClaim: (code: string, auto = false, dispute = false) => {
			claimAuto = auto;
			claimDispute = dispute;
			claimCode = code;
		},
		closeClaim: () => (claimCode = null),
		closeMinigame: () => (minigame = null),
		/** plays an album's tracks (a media item with a player – agents/ITEMS.md "Players, albums, related items") */
		playAlbum: (request: AudioRequest) => (audio = request.tracks.length ? request : null),
		closeAudio: () => (audio = null),
		requestInventory: (value = true) => (inventoryRequested = value)
	};
}

export function setItemsUiContext() {
	const context = createItemsUiContext();
	setContext(ITEMS_UI_KEY, context);
	return context;
}

export function getItemsUiContext() {
	return getContext<ItemsUiContext>(ITEMS_UI_KEY);
}
