#!/usr/bin/env node
// Local dev server for the API with concurrency on Windows (agents/PERFORMANCE.md).
//
//   node server/bin/dev-serve.mjs [--port=<api port>] [--workers=4] [--no-opcache]
//   (default port: "api" in shared/dev-ports.json, the single source of truth)
//   npm run dev:api            (root)      composer serve (server/)
//
// `php -S` handles one request at a time, and PHP_CLI_SERVER_WORKERS needs fork() ("forking is not supported on this
// platform" on Windows). This starts N `php -S` workers on [::1]:<port+200+i> and a small Node proxy on <port> that
// sends each request to the least busy worker, so parallel API calls (a chapter page fires 6+ at once) don't queue
// behind each other. The workers run with OPcache on (validate_timestamps + revalidate_freq=0: edits apply at once),
// which cuts the per-request bootstrap from ~60 ms to ~15 ms. The proxy keeps Host / headers / body as they are and
// talks to the workers over [::1], so PHP still sees REMOTE_ADDR ::1 (as with a plain `php -S localhost:<port>`).
// It listens on both ::1 and 127.0.0.1: `localhost` then answers at once for every client (curl tries 127.0.0.1
// first and waited ~200 ms for the IPv6 fallback when only ::1 listened).
import { spawn, spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import http from 'node:http';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const args = Object.fromEntries(
	process.argv.slice(2).map((a) => {
		const [k, v] = a.replace(/^--/, '').split('=');
		return [k, v ?? true];
	})
);
const port = Number(args.port || process.env.PORT || JSON.parse(readFileSync(new URL('../../shared/dev-ports.json', import.meta.url), 'utf8')).api);
const workers = Math.max(1, Number(args.workers || process.env.API_WORKERS || 4));
const php = process.env.PHP_BINARY || 'php';
const root = join(dirname(fileURLToPath(import.meta.url)), '..');

const loaded = spawnSync(php, ['-r', "echo extension_loaded('Zend OPcache') ? 1 : 0;"], { encoding: 'utf8' }).stdout.trim() === '1';
const opcache = args['no-opcache']
	? []
	: [...(loaded ? [] : ['-d', 'zend_extension=opcache']), '-d', 'opcache.enable_cli=1', '-d', 'opcache.validate_timestamps=1', '-d', 'opcache.revalidate_freq=0'];

const pool = [];
for (let i = 0; i < workers; i++) {
	const workerPort = port + 200 + i;
	const child = spawn(php, [...opcache, '-S', `[::1]:${workerPort}`, '-t', 'public', 'public/router.php'], { cwd: root, stdio: ['ignore', 'inherit', 'pipe'] });
	// php -S logs every request on stderr; keep errors/warnings and the start line, drop the access lines
	child.stderr.setEncoding('utf8');
	child.stderr.on('data', (chunk) => {
		for (const line of chunk.split(/\r?\n/)) {
			if (!line || / (Accepted|Closing)$/.test(line) || /\]:\d+ \[\d{3}\]: /.test(line)) continue;
			process.stderr.write(`[php ${workerPort}] ${line}\n`);
		}
	});
	child.on('exit', (code) => {
		console.error(`[php ${workerPort}] exited (${code}) – stopping`);
		shutdown(1);
	});
	pool.push({ port: workerPort, busy: 0, child });
}

function pick() {
	return pool.reduce((best, w) => (w.busy < best.busy ? w : best), pool[0]);
}

function handle(req, res) {
	const worker = pick();
	worker.busy++;
	let done = false;
	const release = () => {
		if (!done) {
			done = true;
			worker.busy--;
		}
	};
	const upstream = http.request(
		{ host: '::1', port: worker.port, method: req.method, path: req.url, headers: req.headers },
		(up) => {
			res.writeHead(up.statusCode || 502, up.rawHeaders);
			up.pipe(res);
			up.on('end', release);
			up.on('error', release);
		}
	);
	upstream.on('error', (err) => {
		release();
		// the worker may still be starting: one retry after a short wait
		if (!req.retried && err.code === 'ECONNREFUSED' && ['GET', 'HEAD', 'OPTIONS'].includes(req.method || '')) {
			req.retried = true;
			setTimeout(() => handle(req, res), 300);
			return;
		}
		if (!res.headersSent) res.writeHead(502, { 'Content-Type': 'text/plain' });
		res.end(`dev-serve: worker ${worker.port} failed: ${err.message}`);
	});
	res.on('close', release);
	req.pipe(upstream);
}

const servers = ['::1', '127.0.0.1'].map((host) => {
	const server = http.createServer(handle);
	server.keepAliveTimeout = 5000;
	server.on('error', (err) => {
		console.error(`dev-serve: cannot listen on ${host}:${port}: ${err.message}`);
		shutdown(1);
	});
	server.listen(port, host);
	return server;
});
console.log(`API dev server on http://localhost:${port} -> ${workers} php -S workers (${pool.map((w) => w.port).join(', ')}), OPcache ${opcache.length ? 'on' : 'off'}`);

let stopping = false;
function shutdown(code = 0) {
	if (stopping) return;
	stopping = true;
	for (const w of pool) w.child.kill();
	for (const s of servers) s.close();
	setTimeout(() => process.exit(code), 200);
}
process.on('SIGINT', () => shutdown(0));
process.on('SIGTERM', () => shutdown(0));
process.on('exit', () => pool.forEach((w) => w.child.kill()));
