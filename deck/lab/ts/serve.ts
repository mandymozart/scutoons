// `node --experimental-transform-types deck/lab/ts/serve.ts [port]` – serves deck/lab/www on 127.0.0.1 (default 7790)
// with COOP / COEP, so the page is cross-origin isolated (5 µs timers, SharedArrayBuffer for the audio ring). A page
// asked for with `?coi=0` comes without them (demo.html's fallback: live-core's transfer transport).
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../www', import.meta.url));
const port = Number(process.argv[2] ?? 7790);
const types: Record<string, string> = {
	'.html': 'text/html; charset=utf-8',
	'.js': 'text/javascript',
	'.wasm': 'application/wasm',
	'.json': 'application/json',
	'.css': 'text/css'
};

createServer(async (req, res) => {
	const [url, query = ''] = (req.url ?? '/').split('?');
	const path = normalize(decodeURIComponent(url)).replace(/^([/\\])+/, '');
	const isolate = !new URLSearchParams(query).has('coi', '0');
	const file = join(root, path === '' || path === '.' ? 'index.html' : path);
	if (!file.startsWith(root)) return void res.writeHead(403).end();
	try {
		const body = await readFile(file);
		res.writeHead(200, {
			'content-type': types[extname(file)] ?? 'application/octet-stream',
			...(isolate ? { 'cross-origin-opener-policy': 'same-origin', 'cross-origin-embedder-policy': 'require-corp' } : {}),
			'cache-control': 'no-store'
		});
		res.end(body);
	} catch {
		res.writeHead(404).end();
	}
}).listen(port, '127.0.0.1', () => console.log(`lab on http://127.0.0.1:${port}/`));
