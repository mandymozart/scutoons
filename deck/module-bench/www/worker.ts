// worker.ts – one runtime of the #306 suite in a worker (bench.ts `serve`): a browser module worker reads dist/ over
// fetch, a Node worker thread from the disk. One message in, the answer (and the watchdog's `calling`) out.
import { serve } from './bench.ts';

const node = typeof (globalThis as any).process?.versions?.node === 'string';
if (node) {
	const { parentPort } = await import('node:worker_threads');
	const { readFile } = await import('node:fs/promises');
	const dist = new URL('./dist/', import.meta.url);
	parentPort!.once('message', (m) => serve(m, async (n) => new Uint8Array(await readFile(new URL(n, dist))), (r) => parentPort!.postMessage(r)));
} else {
	self.onmessage = (e: MessageEvent) =>
		serve(e.data, async (n) => new Uint8Array(await (await fetch(`dist/${n}`, { cache: 'no-store' })).arrayBuffer()), (r) => (self as any).postMessage(r));
}
