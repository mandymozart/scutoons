#!/usr/bin/env node
// Guards shared/dev-ports.json as the single source of the local dev ports (part of the root `npm run check`):
// - the apps' vite configs and the dev start-up code read it instead of hard-coding a port,
// - the local URLs in the .env / config examples and the PLATFORM.md table match it.
//   npm run check:ports
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PORTS } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const read = (path) => readFileSync(resolve(ROOT, path), 'utf8');
const errors = [];
const lineOf = (text, index) => text.slice(0, index).split('\n').length;
const ports = new Set(Object.values(PORTS));
const APPS = ['viewer', 'creator', 'admin', 'shop', 'embassy'];

// 1. code that must read the file: no port number of the file as a literal
const CODE = [
	...APPS.map((app) => `${app}/vite.config.ts`),
	...APPS.map((app) => `${app}/src/lib/config.ts`),
	'scripts/dev/up.mjs',
	'scripts/dev/launch-json.mjs',
	'server/bin/dev-serve.mjs',
	'server/composer.json',
	'qr/composer.json',
	'package.json'
];
for (const path of CODE) {
	const text = read(path);
	for (const m of text.matchAll(/\b\d{4,5}\b/g)) {
		if (ports.has(Number(m[0]))) errors.push(`${path}:${lineOf(text, m.index)}: hard-coded port ${m[0]} (read shared/dev-ports.json)`);
	}
}
for (const app of APPS) {
	const text = read(`${app}/vite.config.ts`);
	for (const block of ['server', 'preview']) {
		if (!new RegExp(`${block}: \\{[^}]*port: ports\\.${app},[^}]*strictPort: true`).test(text)) {
			errors.push(`${app}/vite.config.ts: ${block}.port must be ports.${app} with strictPort: true`);
		}
	}
}

// 2. examples: every local URL is a port of the file; known variables point at their service
const VARS = { VITE_API_URL: 'api', VITE_QR_API: 'api', VITE_CDN_URL: 'cdn', VITE_WWW_URL: 'viewer', VITE_VIEWER_URL: 'viewer', VITE_CREATOR_URL: 'creator', VITE_SHOP_URL: 'shop', VITE_ADMIN_URL: 'admin', API_URL: 'api', WWW_URL: 'viewer', CDN_URL: 'cdn', QR_URL: 'qr', APP_URL: 'viewer' };
const EXAMPLES = [...APPS.map((app) => `${app}/.env.example`), 'server/config.local.php.example', 'qr/config.local.php.example'];
const LEGACY = new Set([5173, 5174, 5175, 4173]); // old Vite ports, still listed as extra CORS origins
for (const path of EXAMPLES) {
	const text = read(path);
	for (const m of text.matchAll(/(?:\b([A-Z_]+)\b['"]?\s*(?:=|,|=>)\s*['"]?)?https?:\/\/(?:localhost|127\.0\.0\.1):(\d+)/g)) {
		const port = Number(m[2]);
		const at = `${path}:${lineOf(text, m.index)}`;
		if (!ports.has(port) && !LEGACY.has(port)) errors.push(`${at}: port ${port} is not in shared/dev-ports.json`);
		const service = VARS[m[1]];
		if (service && PORTS[service] !== port) errors.push(`${at}: ${m[1]} should use port ${PORTS[service]} (${service})`);
	}
}

// 3. the readable table in agents/PLATFORM.md "Local dev ports"
const platform = read('agents/PLATFORM.md');
const table = platform.slice(platform.indexOf('## Local dev ports')).split('\n## ')[0];
const rows = [...table.matchAll(/^\| (\w+)[^|]*\| \*\*(\d+)\*\* \|/gm)].map((m) => [m[1].toLowerCase(), Number(m[2])]);
for (const [name, port] of rows) {
	if (PORTS[name] !== port) errors.push(`agents/PLATFORM.md "Local dev ports": ${name} ${port}, shared/dev-ports.json says ${PORTS[name] ?? 'nothing'}`);
}
for (const service of Object.keys(PORTS)) {
	if (!rows.some(([name]) => name === service)) errors.push(`agents/PLATFORM.md "Local dev ports": no row for ${service}`);
}

if (errors.length) {
	console.error(`check:ports: ${errors.length} problem(s)\n${errors.map((e) => `  ${e}`).join('\n')}`);
	process.exit(1);
}
console.log(`check:ports: ok (${Object.entries(PORTS).map(([n, p]) => `${n} ${p}`).join(', ')})`);
