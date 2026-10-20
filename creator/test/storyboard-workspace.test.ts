// The member's workspace folder (src/lib/editor/storyboard-workspace.ts, #123; STUDIO-IN-CREATOR §2.2 place 1): a fake
// folder tree stands in for the File System Access API – choose, remember, ask again, read / write / list under
// <toon>/<chapter>, names that can't leave the chapter's folder.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { nameParts, nextGenerationFile, Workspace, WorkspaceError, type DirHandle, type FileHandleLike, type HandleStore } from '../src/lib/editor/storyboard-workspace.ts';

const notFound = () => new DOMException('missing', 'NotFoundError');

class FakeDir implements DirHandle {
	readonly kind = 'directory' as const;
	dirs = new Map<string, FakeDir>();
	files = new Map<string, Blob>();
	permission: PermissionState = 'granted';
	/** what requestPermission answers */
	grant: PermissionState = 'granted';
	constructor(readonly name: string) {}
	async getDirectoryHandle(name: string, o?: { create?: boolean }) {
		let d = this.dirs.get(name);
		if (!d) {
			if (!o?.create) throw notFound();
			this.dirs.set(name, (d = new FakeDir(name)));
		}
		return d;
	}
	async getFileHandle(name: string, o?: { create?: boolean }): Promise<FileHandleLike> {
		if (!this.files.has(name) && !o?.create) throw notFound();
		const files = this.files;
		return {
			kind: 'file',
			name,
			getFile: async () => files.get(name)!,
			createWritable: async () => {
				let data: Blob = new Blob([]);
				return { write: async (b: Blob) => void (data = b), close: async () => void files.set(name, data) };
			}
		};
	}
	async *values() {
		for (const d of this.dirs.values()) yield d;
		for (const name of this.files.keys()) yield (await this.getFileHandle(name)) as FileHandleLike;
	}
	async queryPermission() {
		return this.permission;
	}
	async requestPermission() {
		return (this.permission = this.grant);
	}
}

const memoryStore = (): HandleStore & { held: DirHandle | null } => {
	const s = { held: null as DirHandle | null, get: async () => s.held, set: async (h: DirHandle) => void (s.held = h), clear: async () => void (s.held = null) };
	return s;
};

test('no folder access in the browser: unsupported, and nothing reads or writes', async () => {
	const w = new Workspace({ store: memoryStore(), pick: null });
	assert.equal(w.supported, false);
	assert.equal(await w.restore(), 'unsupported');
	assert.equal(await w.choose(), 'unsupported');
	await assert.rejects(w.list('catpope', 3), (e: unknown) => e instanceof WorkspaceError && e.code === 'unsupported');
});

test('choose once, remembered: the next visit restores it without asking', async () => {
	const root = new FakeDir('SCUTOONS');
	const store = memoryStore();
	const first = new Workspace({ store, pick: async () => root });
	assert.equal(await first.restore(), 'none');
	assert.equal(await first.choose(), 'ready');
	assert.equal(first.name, 'SCUTOONS');
	const next = new Workspace({ store, pick: async () => root });
	assert.equal(await next.restore(), 'ready');
	assert.equal(next.name, 'SCUTOONS');
	await next.forget();
	assert.equal(store.held, null);
});

test('a lapsed permission: "ask" until the member allows it again', async () => {
	const root = new FakeDir('ws');
	const store = memoryStore();
	store.held = root;
	root.permission = 'prompt';
	const w = new Workspace({ store, pick: async () => root });
	assert.equal(await w.restore(), 'ask');
	await assert.rejects(w.read('catpope', 3, 'a.png'), (e: unknown) => e instanceof WorkspaceError && e.code === 'permission');
	root.grant = 'denied';
	assert.equal(await w.allow(), 'ask');
	root.grant = 'granted';
	assert.equal(await w.allow(), 'ready');
});

test('a cancelled picker changes nothing', async () => {
	const w = new Workspace({
		store: memoryStore(),
		pick: async () => {
			throw new DOMException('closed', 'AbortError');
		}
	});
	assert.equal(await w.choose(), null);
	assert.equal(w.name, null);
});

test('write, read and list under <toon>/<chapter>', async () => {
	const root = new FakeDir('ws');
	const w = new Workspace({ store: memoryStore(), pick: async () => root });
	await w.choose();
	assert.deepEqual(await w.list('catpope', 3), []);
	assert.equal(await w.read('catpope', 3, 'sb-25-1--g1.png'), null);
	await w.write('catpope', 3, 'sb-25-1--g1.png', new Blob(['png']));
	await w.write('catpope', 3, 'sb-25-1--g2.mp4', new Blob(['mp4']));
	assert.ok(root.dirs.get('catpope')?.dirs.get('3')?.files.has('sb-25-1--g1.png'));
	assert.equal(await (await w.read('catpope', 3, 'sb-25-1--g2.mp4'))!.text(), 'mp4');
	assert.deepEqual(await w.list('catpope', 3), ['sb-25-1--g1.png', 'sb-25-1--g2.mp4']);
	// another chapter is another folder
	assert.equal(await w.read('catpope', 4, 'sb-25-1--g1.png'), null);
	assert.equal(nextGenerationFile('sb-25-1', await w.list('catpope', 3), 'png'), 'sb-25-1--g3.png');
});

test('a name can never leave the chapter folder', () => {
	assert.deepEqual(nameParts('frames/sb-1.png'), ['frames', 'sb-1.png']);
	for (const bad of ['../x.png', 'a/../b.png', '/abs.png', '', 'a//b.png', '.hidden']) assert.throws(() => nameParts(bad), WorkspaceError, bad);
});
