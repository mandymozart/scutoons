// The creator's copy / cut / paste runtime (BACKLOG 10.39, agents/UI-CONCEPTS.md E.7, DESIGN.md "Copy / paste"):
// Cmd/Ctrl+C / X / V act on the LIVE region's selection only (@scutoons/ui liveRegion, E.8). Each region that can copy
// registers a handler under its region name (`useClipHandler`); the DOM copy / cut / paste events (no permission
// prompt) carry the clip as JSON on the system clipboard (clip.ts). Shift+V (Ctrl/Cmd+Shift+V) pastes the settings
// only onto the selection. In a text field, or with page text selected, the keys stay the browser's.
// A paste is ONE doc.edit (one undo step), saved by the normal PUT. The menu (phones, right-click – clip-menu.svelte)
// calls `clipboard.copy / paste` directly: copy writes the system clipboard too, paste reads it when the browser
// allows, else the clip copied last in this tab.
import { liveRegion } from '@scutoons/ui';
import { isField } from '$lib/editor/editor-keys.ts';
import { clipText, foreignFiles, readClip, type Clip } from '$lib/editor/clip.ts';
import { toast } from '$lib/toasts.ts';
import { shared } from '$lib/utils/shared.ts';
import { m } from '$lib/paraglide/messages.js';

/** what a paste did: the toast's text; `error` = refused (the toast stays until dismissed) */
export interface ClipResult {
	message: string;
	error?: boolean;
}

export interface ClipHandler {
	/** the live selection as a clip + its words ("text box", "events (3)"); null: nothing selected */
	copy(): { clip: Clip; what: string } | null;
	/** cut = copy + remove the selection (one undo step) */
	remove(): void;
	/** pastes as new; null: this region doesn't take that kind of clip */
	paste(clip: Clip): ClipResult | null;
	/** Shift+V: the clip's settings onto the selection; null: not for this kind */
	pasteSettings?(clip: Clip): ClipResult | null;
}

/** the words of a clip ("text box", "events (3)") */
export function clipWhat(clip: Pick<Clip, 'kind' | 'entries'>): string {
	const count = clip.entries.length;
	switch (clip.kind) {
		case 'dialog':
			return m['creator.clip_what_dialog']();
		case 'item':
			return m['creator.clip_what_item']();
		case 'piece':
			return m['creator.clip_what_piece']();
		case 'sample':
			return m['creator.clip_what_sample']();
		case 'event':
			return m['creator.clip_what_event']();
		case 'events':
			return m['creator.clip_what_events']({ count });
		case 'slices':
			return m['creator.clip_what_slices']({ count });
		case 'groups':
			return m['creator.clip_what_groups']({ count });
	}
}

/** "Pasted: text box" (+ " – uses 2 files from catpope" across toons: they are referenced, never copied) */
export function pastedMessage(clip: Clip, toon: string, settings = false, what = clipWhat(clip)): string {
	const message = settings ? m['creator.clip_pasted_settings']({ what }) : m['creator.clip_pasted']({ what });
	// a piece's settings are its look (preset + style – clip.ts applyPieceLook): its file stays the target's own
	const files = settings && clip.kind === 'piece' ? 0 : foreignFiles(clip.entries, clip.sounds, clip.toon, toon, clip.kind);
	return files ? m['creator.clip_foreign_files']({ message, count: files, toon: clip.toon }) : message;
}

class CreatorClipboard {
	/** the clip copied last in this tab: the menu's paste when the browser won't hand out the system clipboard */
	last = $state<Clip | null>(null);
	#handlers = new Map<string, ClipHandler>();

	/** registers the handler of a region; returns the unregister function */
	register(region: string, handler: ClipHandler): () => void {
		this.#handlers.set(region, handler);
		return () => {
			if (this.#handlers.get(region) === handler) this.#handlers.delete(region);
		};
	}

	/** the live region's handler (null: none live / not a copy region) */
	get handler(): ClipHandler | null {
		return liveRegion.current ? (this.#handlers.get(liveRegion.current) ?? null) : null;
	}

	/** has the region a handler (the right-click menu opens only there) */
	handles(region: string | null | undefined): boolean {
		return !!region && this.#handlers.has(region);
	}

	/** copies (cut: and removes) the live selection; the clip, null when nothing is selected */
	copy(cut = false): Clip | null {
		const handler = this.handler;
		const taken = handler?.copy();
		if (!handler || !taken) return null;
		this.last = taken.clip;
		if (cut) handler.remove();
		toast(cut ? m['creator.clip_cut']({ what: taken.what }) : m['creator.clip_copied']({ what: taken.what }), { type: 'success' });
		return taken.clip;
	}

	/** pastes a clip into the live region (settingsOnly: Shift+V) */
	paste(clip: Clip, settingsOnly = false) {
		const handler = this.handler;
		const result = handler ? (settingsOnly ? (handler.pasteSettings?.(clip) ?? null) : handler.paste(clip)) : null;
		if (!result) {
			toast(handler ? m['creator.clip_wrong_place']({ what: clipWhat(clip) }) : m['creator.clip_no_target'](), { type: 'warning' });
			return;
		}
		toast(result.message, result.error ? { type: 'error', timeout: 0 } : { type: 'success' });
	}

	/** the menu's copy: also onto the system clipboard (a user gesture – no prompt) */
	async copyFromMenu(cut = false) {
		const clip = this.copy(cut);
		if (!clip) return;
		try {
			await navigator.clipboard.writeText(clipText(clip));
		} catch {
			/* the tab's own copy (`last`) still pastes */
		}
	}

	/** the menu's paste: the system clipboard when the browser allows, else the clip copied last in this tab */
	async pasteFromMenu(settingsOnly = false) {
		let clip: Clip | null = null;
		try {
			clip = readClip(await navigator.clipboard.readText());
		} catch {
			/* no permission / not supported */
		}
		clip ??= this.last;
		if (!clip) {
			toast(m['creator.clip_empty'](), { type: 'warning' });
			return;
		}
		this.paste(clip, settingsOnly);
	}
}

/** the creator's one clipboard */
export const clipboard = shared('clipboard', () => new CreatorClipboard());

/** registers a region's handler while the calling component lives (call during component init) */
export function useClipHandler(region: () => string | null, handler: ClipHandler) {
	$effect(() => {
		const name = region();
		if (!name) return;
		return clipboard.register(name, handler);
	});
}

/** page text is selected: Ctrl+C copies that, as in any page */
const textSelected = () => {
	const selection = typeof window !== 'undefined' ? window.getSelection() : null;
	return !!selection && !selection.isCollapsed && selection.toString().trim() !== '';
};

/**
 * The DOM copy / cut / paste events of the page (attach once, e.g. `$effect(() => clipboardEvents())`); returns the
 * cleanup. Ctrl/Cmd+Shift+V marks the next paste as "settings only".
 */
export function clipboardEvents(): () => void {
	let shiftPasteAt = 0;
	const onKeydown = (event: KeyboardEvent) => {
		if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === 'v') shiftPasteAt = Date.now();
	};
	const onCopy = (event: ClipboardEvent, cut: boolean) => {
		if (isField(event.target) || textSelected()) return;
		const clip = clipboard.copy(cut);
		if (!clip) return;
		event.clipboardData?.setData('text/plain', clipText(clip));
		event.preventDefault();
	};
	const copy = (event: ClipboardEvent) => onCopy(event, false);
	const cut = (event: ClipboardEvent) => onCopy(event, true);
	const paste = (event: ClipboardEvent) => {
		if (isField(event.target)) return;
		const clip = readClip(event.clipboardData?.getData('text/plain'));
		if (!clip) return;
		event.preventDefault();
		const settingsOnly = Date.now() - shiftPasteAt < 1000;
		shiftPasteAt = 0;
		clipboard.paste(clip, settingsOnly);
	};
	document.addEventListener('keydown', onKeydown, true);
	document.addEventListener('copy', copy);
	document.addEventListener('cut', cut);
	document.addEventListener('paste', paste);
	return () => {
		document.removeEventListener('keydown', onKeydown, true);
		document.removeEventListener('copy', copy);
		document.removeEventListener('cut', cut);
		document.removeEventListener('paste', paste);
	};
}
