// Local CDN for development: serves media/ (the dev media storage, ASSET_LOCAL_ROOT) with CORS on
// http://localhost:<port> (default: "cdn" in shared/dev-ports.json, the single source of the local dev ports).
//
//   node scripts/dev/media-serve.mjs [port] [--root=<folder>]
//
// Loopback only, on both localhost addresses (the apps' `localhost` resolves to ::1 first). Idle sockets are closed
// after 30 s (Chrome preconnects / keep-alive must never block other requests – agents/PERFORMANCE.md). Answers single
// HTTP Range requests (`206 Partial Content`, `416` when unsatisfiable) so videos can seek; production (bunny) serves
// ranges natively. Logs errors only (a chapter page loads 100+ files).
import { createReadStream, statSync } from 'node:fs';
import { createServer } from 'node:http';
import { dirname, extname, join, normalize, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PORTS } from './ports.mjs';

const args = process.argv.slice(2);
const port = Number(args.find((a) => /^\d+$/.test(a)) ?? PORTS.cdn);
const root = resolve(args.find((a) => a.startsWith('--root='))?.slice(7) ?? join(dirname(fileURLToPath(import.meta.url)), '../../media'));

const TYPES = {
	'.webp': 'image/webp', '.png': 'image/png', '.jpg': 'image/jpeg', '.jpeg': 'image/jpeg', '.gif': 'image/gif',
	'.svg': 'image/svg+xml', '.avif': 'image/avif', '.ico': 'image/x-icon',
	'.mp4': 'video/mp4', '.webm': 'video/webm', '.mp3': 'audio/mpeg', '.m4a': 'audio/mp4', '.ogg': 'audio/ogg', '.wav': 'audio/wav',
	'.json': 'application/json', '.bin': 'application/octet-stream', '.pdf': 'application/pdf', '.zip': 'application/zip',
	'.txt': 'text/plain; charset=utf-8', '.html': 'text/html; charset=utf-8', '.css': 'text/css', '.js': 'text/javascript',
	'.woff2': 'font/woff2', '.woff': 'font/woff', '.ttf': 'font/ttf',
};

const CORS = {
	'Access-Control-Allow-Origin': '*',
	'Access-Control-Allow-Methods': 'GET, HEAD, OPTIONS',
	'Access-Control-Allow-Headers': 'Content-Type, Range',
	'Access-Control-Expose-Headers': 'Content-Length, Content-Range, Accept-Ranges',
	'Accept-Ranges': 'bytes',
};

function fail(req, res, status) {
	console.error(`${status} ${req.method} ${req.url}`);
	res.writeHead(status, CORS).end();
}

function handle(req, res) {
	if (req.method === 'OPTIONS') return res.writeHead(204, CORS).end();
	if (req.method !== 'GET' && req.method !== 'HEAD') return fail(req, res, 405);
	let path;
	try {
		path = normalize(join(root, decodeURIComponent(new URL(req.url, 'http://x').pathname)));
	} catch {
		return fail(req, res, 400);
	}
	if (path !== root && !path.startsWith(root + sep)) return fail(req, res, 403);
	let stat;
	try {
		stat = statSync(path);
	} catch {
		return fail(req, res, 404);
	}
	if (!stat.isFile()) return fail(req, res, 404);

	const size = stat.size;
	const headers = { ...CORS, 'Content-Type': TYPES[extname(path).toLowerCase()] ?? 'application/octet-stream', 'Last-Modified': stat.mtime.toUTCString() };
	let start = 0;
	let end = size - 1;
	let status = 200;
	const range = /^bytes=(\d*)-(\d*)$/.exec(String(req.headers.range ?? '').trim());
	if (range && (range[1] || range[2])) {
		if (range[1]) {
			start = Number(range[1]);
			end = range[2] ? Math.min(Number(range[2]), size - 1) : size - 1;
		} else start = Math.max(0, size - Number(range[2])); // suffix range: the last N bytes
		if (start >= size || start > end) return res.writeHead(416, { ...CORS, 'Content-Range': `bytes */${size}`, 'Content-Length': 0 }).end();
		status = 206;
		headers['Content-Range'] = `bytes ${start}-${end}/${size}`;
	}
	headers['Content-Length'] = end - start + 1;
	res.writeHead(status, headers);
	if (req.method === 'HEAD' || size === 0) return res.end();
	createReadStream(path, { start, end }).on('error', () => res.destroy()).pipe(res);
}

let listening = 0;
for (const host of ['127.0.0.1', '::1']) {
	const server = createServer(handle);
	server.keepAliveTimeout = 30_000;
	server.headersTimeout = 31_000;
	server.requestTimeout = 0; // long video downloads
	server.on('error', (error) => {
		if (host === '::1' && error.code === 'EADDRNOTAVAIL') return; // no IPv6 loopback
		console.error(`media server on ${host}:${port}: ${error.message}`);
		process.exit(1);
	});
	server.listen(port, host, () => {
		if (++listening === 1) console.log(`Media server on http://localhost:${port}, serving ${root}`);
	});
}
