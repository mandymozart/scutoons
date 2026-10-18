// The vector library → the LOCAL asset manager (agents/research/VECTORS.md §6 step 5), as the test user
// (npm run dev:test-user): scripts/vectors-core.mjs does the work – svgo, upload to `catpope/vectors/<slug>.svg`, the
// shape rows. Locally a shape with other bytes is replaced (the dev copy). Production never goes through this script:
// the SCUTOONS MCP tool `import_vectors` runs the same core as the signed-in member and never replaces a file.
//   node scripts/vectors-import.mjs [--dry-run] [slug …]
import fs from 'node:fs';
import path from 'node:path';
import { importVectors } from './vectors-core.mjs';
import { local } from './dev/ports.mjs';

const root = path.resolve(import.meta.dirname, '..');
const args = process.argv.slice(2);
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
if (!/^http:\/\/(localhost|127\.0\.0\.1)(:\d+)?\//.test(API + '/')) throw new Error(`refusing ${API}: this script runs against the local dev API only – production: the MCP tool import_vectors`);
const session = path.join(root, 'logs/dev/test-user.json');
if (!fs.existsSync(session)) throw new Error('no test-user session: npm run dev:test-user');
const { token } = JSON.parse(fs.readFileSync(session, 'utf8'));

async function call(method, url, body, type = 'application/json') {
	const res = await fetch(API + url, {
		method,
		headers: { Authorization: `Bearer ${token}`, ...(body !== undefined ? { 'Content-Type': type } : {}) },
		body: body === undefined ? undefined : type === 'application/json' ? JSON.stringify(body) : body
	});
	const json = await res.json().catch(() => ({}));
	if (!res.ok) throw new Error(`${method} ${url} → ${res.status} ${json.error?.code ?? json.code ?? ''} ${json.error?.message ?? json.message ?? ''}`.trim());
	return json;
}
const q = encodeURIComponent;

const result = await importVectors({
	dryRun: args.includes('--dry-run'),
	only: new Set(args.filter((a) => !a.startsWith('--'))),
	log: console.log,
	async find(key) {
		const { asset } = await call('GET', `/assets/lookup/?path=${q(key)}`);
		if (!asset) return null;
		const current = asset.versions.find((v) => v.status === 'current');
		return { id: asset.asset.id, sha256: current?.sha256 ?? null, title: asset.asset.title, tags: asset.asset.tags, shape: asset.asset.shape ?? null };
	},
	upload: async (bytes, key, title, tags) => (await call('PUT', `/assets/upload/?path=${q(key)}&title=${q(title)}&tags=${q(tags.join(','))}`, bytes, 'image/svg+xml')).asset.id,
	replaceFile: async (bytes, key) => void (await call('PUT', `/assets/replace/?path=${q(key)}&note=vectors-import`, bytes, 'image/svg+xml')),
	save: async (id, body) => void (await call('PUT', `/assets/${id}`, body))
});
console.log(JSON.stringify(result));
if (result.failed.length || result.collision.length) process.exitCode = 1;
