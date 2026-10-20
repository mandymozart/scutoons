// The member's workspace folder (#123; STUDIO-IN-CREATOR §2.2 place 1): every attempt lives on the member's own disk –
// generated stills and videos, rendered frames, takes – and is never uploaded by itself. The member picks the folder
// once on the Storyboard page (the File System Access API: Chrome / Edge); its handle is kept in IndexedDB and the
// permission asked again when the browser has let it lapse. Layout: <folder>/<toon>/<chapter>/<file>, a generation's
// file `<shot>--g<n>.<ext>` (@scutoons/content nextName). The MCP extension is set to the same folder (§2.4).
// Everything goes through two small interfaces (`DirHandle`, `HandleStore`), so the logic is tested with a fake.
import { nextName } from '@scutoons/content/storyboard';

/** the part of a FileSystemFileHandle this module uses */
export interface FileHandleLike {
	readonly kind: 'file';
	readonly name: string;
	getFile(): Promise<Blob>;
	createWritable(): Promise<{ write(data: Blob): Promise<void>; close(): Promise<void> }>;
}

/** the part of a FileSystemDirectoryHandle this module uses */
export interface DirHandle {
	readonly kind: 'directory';
	readonly name: string;
	getDirectoryHandle(name: string, options?: { create?: boolean }): Promise<DirHandle>;
	getFileHandle(name: string, options?: { create?: boolean }): Promise<FileHandleLike>;
	values(): AsyncIterable<FileHandleLike | DirHandle>;
	queryPermission?(descriptor: { mode: 'readwrite' }): Promise<PermissionState>;
	requestPermission?(descriptor: { mode: 'readwrite' }): Promise<PermissionState>;
}

/** where the chosen folder's handle is kept between visits (IndexedDB; a fake in the tests) */
export interface HandleStore {
	get(): Promise<DirHandle | null>;
	set(handle: DirHandle): Promise<void>;
	clear(): Promise<void>;
}

/**
 * - `unsupported`: the browser has no folder access (Firefox, Safari, phones)
 * - `none`: no folder chosen yet
 * - `ask`: a folder is remembered, but the browser wants the member's OK again (a click on "Allow")
 * - `ready`: read and write
 */
export type WorkspaceState = 'unsupported' | 'none' | 'ask' | 'ready';

export class WorkspaceError extends Error {
	constructor(
		readonly code: 'unsupported' | 'none' | 'permission' | 'name',
		message: string
	) {
		super(message);
	}
}

/** a file name inside the chapter's folder: plain parts, no `..`, no absolute path */
const PART = /^[A-Za-z0-9_][A-Za-z0-9 ._()+-]{0,190}$/;
export function nameParts(name: string): string[] {
	const parts = String(name).split('/');
	if (!parts.length || parts.some((p) => !PART.test(p) || p === '.' || p === '..' || p.includes('..'))) throw new WorkspaceError('name', `Not a file name of the workspace: ${name}`);
	return parts;
}

/** the chapter's folder inside the workspace: <toon>/<chapter> */
export const chapterFolder = (toon: string, chapter: number) => [toon, String(chapter)];

/** the next free file of a shot's generation: `<shot>--g<n>.<ext>` after the names already there */
export const nextGenerationFile = (shot: string, names: string[], ext: string): string => nextName(shot, names, ext).file ?? `${shot}--g1.${ext}`;

const MODE = { mode: 'readwrite' } as const;

export class Workspace {
	handle: DirHandle | null = null;

	constructor(
		private readonly deps: {
			store: HandleStore;
			/** the browser's folder picker (`showDirectoryPicker`); null where there is none */
			pick: (() => Promise<DirHandle>) | null;
		}
	) {}

	get supported(): boolean {
		return !!this.deps.pick;
	}

	/** the folder's name (shown in the Workspace control) */
	get name(): string | null {
		return this.handle?.name ?? null;
	}

	private async permission(request: boolean): Promise<PermissionState> {
		const h = this.handle;
		if (!h) return 'denied';
		const ask = request ? h.requestPermission : h.queryPermission;
		// a handle without the permission methods (an older engine) works as long as it is held
		return ask ? ask.call(h, MODE) : 'granted';
	}

	/** the remembered folder and whether it can be used now (a page load – never asks the member) */
	async restore(): Promise<WorkspaceState> {
		if (!this.supported) return 'unsupported';
		try {
			this.handle = await this.deps.store.get();
		} catch {
			this.handle = null;
		}
		if (!this.handle) return 'none';
		return (await this.permission(false)) === 'granted' ? 'ready' : 'ask';
	}

	/** the member picks the folder (a click): remembered for the next visits; null = the picker was cancelled */
	async choose(): Promise<WorkspaceState | null> {
		if (!this.deps.pick) return 'unsupported';
		let handle: DirHandle;
		try {
			handle = await this.deps.pick();
		} catch (e) {
			// the member closed the picker
			if (e instanceof DOMException && e.name === 'AbortError') return null;
			throw e;
		}
		this.handle = handle;
		await this.deps.store.set(handle);
		return (await this.permission(true)) === 'granted' ? 'ready' : 'ask';
	}

	/** the browser's OK again for the remembered folder (a click) */
	async allow(): Promise<WorkspaceState> {
		if (!this.supported) return 'unsupported';
		if (!this.handle) return 'none';
		return (await this.permission(true)) === 'granted' ? 'ready' : 'ask';
	}

	async forget(): Promise<void> {
		this.handle = null;
		await this.deps.store.clear();
	}

	/** the chapter's folder (created on a write) */
	private async folder(toon: string, chapter: number, create: boolean): Promise<DirHandle | null> {
		if (!this.supported) throw new WorkspaceError('unsupported', 'This browser has no folder access.');
		if (!this.handle) throw new WorkspaceError('none', 'No workspace folder chosen.');
		if ((await this.permission(false)) !== 'granted') throw new WorkspaceError('permission', 'The workspace folder needs the browser’s permission again.');
		let dir = this.handle;
		for (const part of chapterFolder(toon, chapter)) {
			try {
				dir = await dir.getDirectoryHandle(nameParts(part)[0], { create });
			} catch (e) {
				if (!create && e instanceof DOMException && e.name === 'NotFoundError') return null;
				throw e;
			}
		}
		return dir;
	}

	/** a file of the chapter's folder; null = not there */
	async read(toon: string, chapter: number, name: string): Promise<Blob | null> {
		const parts = nameParts(name);
		let dir = await this.folder(toon, chapter, false);
		try {
			for (const part of parts.slice(0, -1)) dir = dir ? await dir.getDirectoryHandle(part) : null;
			if (!dir) return null;
			return await (await dir.getFileHandle(parts[parts.length - 1])).getFile();
		} catch (e) {
			if (e instanceof DOMException && (e.name === 'NotFoundError' || e.name === 'TypeMismatchError')) return null;
			throw e;
		}
	}

	/** writes (or replaces) a file of the chapter's folder */
	async write(toon: string, chapter: number, name: string, data: Blob): Promise<void> {
		const parts = nameParts(name);
		let dir = (await this.folder(toon, chapter, true))!;
		for (const part of parts.slice(0, -1)) dir = await dir.getDirectoryHandle(part, { create: true });
		const file = await (await dir.getFileHandle(parts[parts.length - 1], { create: true })).createWritable();
		try {
			await file.write(data);
		} finally {
			await file.close();
		}
	}

	/** the file names directly in the chapter's folder, sorted */
	async list(toon: string, chapter: number): Promise<string[]> {
		const dir = await this.folder(toon, chapter, false);
		if (!dir) return [];
		const names: string[] = [];
		for await (const entry of dir.values()) if (entry.kind === 'file') names.push(entry.name);
		return names.sort();
	}
}

// ── The browser's parts ─────────────────────────────────────────────────────────────────────────────────────

const DB = 'scutoons-creator';
const STORE = 'handles';
const KEY = 'storyboard-workspace';

function openDb(): Promise<IDBDatabase> {
	return new Promise((resolve, reject) => {
		const req = indexedDB.open(DB, 1);
		req.onupgradeneeded = () => req.result.createObjectStore(STORE);
		req.onsuccess = () => resolve(req.result);
		req.onerror = () => reject(req.error);
	});
}

async function inStore<T>(mode: IDBTransactionMode, run: (store: IDBObjectStore) => IDBRequest): Promise<T> {
	const db = await openDb();
	try {
		return await new Promise<T>((resolve, reject) => {
			const req = run(db.transaction(STORE, mode).objectStore(STORE));
			req.onsuccess = () => resolve(req.result as T);
			req.onerror = () => reject(req.error);
		});
	} finally {
		db.close();
	}
}

/** the handle in this browser's IndexedDB (a handle is structured-cloneable; localStorage can't hold one) */
export const indexedDbStore: HandleStore = {
	get: async () => (typeof indexedDB === 'undefined' ? null : ((await inStore<DirHandle | undefined>('readonly', (s) => s.get(KEY))) ?? null)),
	set: async (handle) => void (await inStore('readwrite', (s) => s.put(handle, KEY))),
	clear: async () => void (await inStore('readwrite', (s) => s.delete(KEY)))
};

/** the browser's workspace: the folder picker where there is one (Chrome / Edge on a desktop) */
export function browserWorkspace(): Workspace {
	const w = typeof window === 'undefined' ? null : (window as unknown as { showDirectoryPicker?: (o: object) => Promise<DirHandle> });
	const picker = w?.showDirectoryPicker;
	return new Workspace({ store: indexedDbStore, pick: picker ? () => picker.call(w, { id: 'scutoons-workspace', mode: 'readwrite' }) : null });
}
