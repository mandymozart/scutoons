import { setContext, getContext } from 'svelte';
import type { Entry, ScanObjectSkill, Skill } from '$lib/models/items.ts';

const ITEMS_UI_KEY = Symbol('items-ui');

export interface CameraRequest {
	qr: boolean;
	object: ScanObjectSkill | null;
	/** entry whose skill opened the camera (for the header) */
	entry: Entry | null;
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
	let minigame = $state<{ id: string; entry: Entry | null } | null>(null);
	let audio = $state<{ src: string; entry: Entry | null } | null>(null);
	let inventoryRequested = $state(false);

	/** Run all "use" skills of an item. Camera skills are merged into one camera session. */
	function use(entry: Entry | null, skills: Skill[]) {
		const qr = skills.some((s) => s.type === 'scan_qr');
		const object = (skills.find((s) => s.type === 'scan_object') as ScanObjectSkill | undefined) ?? null;
		if (qr || object) {
			camera = { qr, object, entry };
			return;
		}
		const game = skills.find((s) => s.type === 'minigame');
		if (game && game.type === 'minigame') {
			minigame = { id: game.id, entry };
			return;
		}
		const play = skills.find((s) => s.type === 'play_audio');
		if (play && play.type === 'play_audio') audio = { src: play.src, entry };
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
		openClaim: (code: string, auto = false) => {
			claimAuto = auto;
			claimCode = code;
		},
		closeClaim: () => (claimCode = null),
		closeMinigame: () => (minigame = null),
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
