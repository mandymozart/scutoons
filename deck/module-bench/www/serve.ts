// serve.ts – the #306 bench page for this PC's browser and the phones on the LAN (MODULE-BENCH.md):
//   node --experimental-strip-types deck/module-bench/www/serve.ts [--port 8100] [--lan]
// Plain HTTP on 127.0.0.1 (localhost is a secure context); --lan: HTTPS on the LAN address with the self-signed
// certificate of `npm run dev:lan` (.dev-certs/lan-<ip>, made with openssl when missing) – a phone needs a secure
// context for `crossOriginIsolated` (the 5 µs clock). COOP / COEP on every answer. `.ts` files are served as
// JavaScript (Node's type stripping, no build step). `POST /results?label=x` writes deck/module-bench/results/x.json.
// Ports: the LAB range 8100+, never the owner's fixed dev ports (shared/dev-ports.json). Ctrl+C stops it.
import { createServer as http, type IncomingMessage, type ServerResponse } from 'node:http';
import { createServer as https } from 'node:https';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { networkInterfaces } from 'node:os';
import { join, normalize, extname, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { stripTypeScriptTypes } from 'node:module';

const www = dirname(fileURLToPath(import.meta.url));
const results = join(www, '..', 'results');
const arg = (name: string, fallback: string) => {
	const i = process.argv.indexOf(`--${name}`);
	return i > 0 ? process.argv[i + 1] : fallback;
};
const port = Number(arg('port', '8100'));
const lan = process.argv.includes('--lan');
const TYPES: Record<string, string> = { '.html': 'text/html', '.ts': 'text/javascript', '.js': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json', '.exe': 'application/octet-stream', '': 'application/octet-stream' };

async function handle(req: IncomingMessage, res: ServerResponse) {
	const url = new URL(req.url ?? '/', 'http://x');
	const head = { 'Cross-Origin-Opener-Policy': 'same-origin', 'Cross-Origin-Embedder-Policy': 'require-corp', 'Cross-Origin-Resource-Policy': 'same-origin', 'Cache-Control': 'no-store' };
	if (req.method === 'POST' && url.pathname === '/results') {
		const chunks: Buffer[] = [];
		for await (const c of req) chunks.push(c as Buffer);
		const label = (url.searchParams.get('label') ?? 'device').replace(/[^a-z0-9._-]/gi, '_');
		JSON.parse(Buffer.concat(chunks).toString('utf8'));
		await mkdir(results, { recursive: true });
		await writeFile(join(results, `${label}.json`), Buffer.concat(chunks));
		console.log(`results ← ${label}.json (${req.socket.remoteAddress})`);
		res.writeHead(200, { ...head, 'content-type': 'text/plain' }).end(`results/${label}.json`);
		return;
	}
	const path = normalize(join(www, url.pathname === '/' ? 'index.html' : decodeURIComponent(url.pathname)));
	if (!path.startsWith(www) || !existsSync(path)) {
		res.writeHead(404, head).end('not found');
		return;
	}
	let body: Buffer | string = await readFile(path);
	if (path.endsWith('.ts')) body = stripTypeScriptTypes(body.toString('utf8'), { mode: 'strip' });
	res.writeHead(200, { ...head, 'content-type': TYPES[extname(path)] ?? 'application/octet-stream' }).end(body);
}

/** the LAN address: a private IPv4, 192.168.* first (as scripts/dev/lan.mjs picks it) */
function lanAddress(): string | undefined {
	const all = Object.entries(networkInterfaces())
		.filter(([name]) => !/vethernet|wsl|docker|virtualbox|vmware|hyper-v/i.test(name))
		.flatMap(([, list]) => list ?? [])
		.filter((a) => a.family === 'IPv4' && !a.internal)
		.map((a) => a.address);
	const rank = (ip: string) => (ip.startsWith('192.168.') ? 0 : ip.startsWith('10.') ? 1 : 2);
	return all.sort((a, b) => rank(a) - rank(b))[0];
}

const serve = (req: IncomingMessage, res: ServerResponse) => handle(req, res).catch((e) => res.writeHead(500).end(String(e)));
if (lan) {
	const ip = arg('ip', lanAddress() ?? '');
	// the main checkout's .dev-certs (a worktree shares it): the git common dir's parent
	const root = dirname(execFileSync('git', ['rev-parse', '--path-format=absolute', '--git-common-dir'], { cwd: www, encoding: 'utf8' }).trim());
	const base = join(root, '.dev-certs', `lan-${ip}`);
	if (!existsSync(`${base}.crt`)) {
		mkdirSync(join(root, '.dev-certs'), { recursive: true });
		execFileSync('openssl', ['req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-sha256', '-days', '825', '-subj', `/CN=SCUTOONS dev ${ip}`, '-addext', `subjectAltName=IP:${ip},DNS:localhost`, '-keyout', `${base}.key`, '-out', `${base}.crt`], { stdio: 'ignore' });
	}
	https({ key: readFileSync(`${base}.key`), cert: readFileSync(`${base}.crt`) }, serve).listen(port, '0.0.0.0', () => {
		console.log(`module bench on the LAN: https://${ip}:${port}/   (first visit: the certificate warning → Advanced → proceed)`);
		console.log(`Windows firewall, once (elevated PowerShell): New-NetFirewallRule -DisplayName "SCUTOONS bench ${port}" -Direction Inbound -Protocol TCP -LocalPort ${port}-${port + 1} -Profile Private -Action Allow`);
	});
	// plain HTTP next to it: the native binary of another machine downloads itself and posts its results here
	http(serve).listen(port + 1, '0.0.0.0', () => console.log(`native binary: http://${ip}:${port + 1}/dist/module-bench.exe · results to http://${ip}:${port + 1}/results?label=<device>-native`));
} else {
	http(serve).listen(port, '127.0.0.1', () => console.log(`module bench: http://127.0.0.1:${port}/`));
}
