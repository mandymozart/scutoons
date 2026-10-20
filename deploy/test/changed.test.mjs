// The deploy planner's file filter (deploy/live.mjs relevantFiles, issue 73): which changed files redeploy which target.
// The rule that matters most: a real change never misses its app (no false negatives) – when in doubt, deploy.
//   node --test deploy/test/*.test.mjs
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { config, globToRegExp, packageClosure, relevantFiles, TARGETS_FILE } from '../live.mjs';

const target = (name) => ({ name, ...config.targets[name] });
const deploys = (files, oldConfig = JSON.stringify(config)) =>
	config.order.filter((name) => relevantFiles(target(name), files, { oldConfig }).length);

test('globs: ** spans directories, * stays in one segment', () => {
	assert.ok(globToRegExp('shared/ui/**').test('shared/ui/src/a/b.ts'));
	assert.ok(globToRegExp('**/*.md').test('README.md'));
	assert.ok(globToRegExp('**/*.md').test('shared/ui/README.md'));
	assert.ok(globToRegExp('*/test/**').test('viewer/test/a.test.ts'));
	assert.ok(!globToRegExp('*/test/**').test('viewer/src/routes/test/+page.svelte'));
	assert.ok(!globToRegExp('shared/ui/**').test('shared/uikit/x.ts'));
	assert.ok(!globToRegExp('package.json').test('viewer/package.json'));
});

test('a shared package reaches exactly the apps that import it (transitively)', () => {
	assert.deepEqual(deploys(['shared/catalog/src/index.ts']), ['creator', 'admin']);
	assert.deepEqual(deploys(['shared/reader/src/page.svelte']), ['viewer', 'creator']);
	assert.deepEqual(deploys(['shared/tokens/tokens.css']), ['qr', 'viewer', 'shop', 'creator', 'admin', 'embassy']);
	// content is imported by every frontend (directly or through api / auth) and by the MCP bundle the api ships
	assert.deepEqual(deploys(['shared/content/src/content.ts']), ['api', 'viewer', 'shop', 'creator', 'admin', 'embassy']);
	// the api ships the member's MCP bundle (issue 76: its build runs tools/mcp/scripts/build.mjs), which reads
	// creator's text presets and the vector import; the viewer no longer does
	assert.deepEqual(deploys(['tools/mcp/src/server.ts']), ['api']);
	assert.deepEqual(deploys(['tools/mcp/scripts/build.mjs']), ['api']);
	assert.deepEqual(deploys(['creator/src/lib/editor/text-presets.ts']), ['api', 'creator']);
	assert.deepEqual(deploys(['scripts/vectors-core.mjs']), ['api']);
});

test('root build files and the deploy machinery still reach every app', () => {
	// the api too: its deploy builds the MCP bundle with the workspace's packages
	assert.deepEqual(deploys(['package-lock.json']), ['api', 'viewer', 'shop', 'creator', 'admin', 'embassy']);
	assert.deepEqual(deploys(['deploy/deploy.mjs']), config.order);
	assert.deepEqual(deploys(['shared/inlang/plugin-message-format.js']), ['viewer', 'shop', 'creator', 'admin', 'embassy']);
});

test('tests, docs, seed and scripts deploy nothing – imported markdown does', () => {
	assert.deepEqual(
		deploys(['shared/ui/test/timeline.test.ts', 'shared/content/scripts/make-sandbox.ts', 'shared/content/seed/x.json', 'shared/ui/README.md', 'server/tests/AuthTest.php', 'server/docs/API.md', 'server/db/prod-seed.sql', 'viewer/test/x.test.ts', 'agents/HANDOVER.md', 'storyboards/style/a.webp', 'scripts/check-map.mjs']),
		[]
	);
	assert.deepEqual(deploys(['admin/src/lib/qr/templates/cos.md']), ['admin']);
	assert.deepEqual(deploys(['server/src/Routes.php', 'viewer/README.md']), ['api']);
});

test('deploy/targets.json counts only for the targets whose block changed', () => {
	const old = structuredClone(config);
	old.targets.viewer.remote = 'old/';
	old.$comment = 'comments never count';
	old.targets.api.$comment = 'neither inside a block';
	assert.deepEqual(deploys([TARGETS_FILE], JSON.stringify(old)), ['viewer']);
	assert.deepEqual(deploys([TARGETS_FILE], JSON.stringify(config)), []);
	// a shared key (protocols) concerns everyone; an unreadable old version too (err on the side of deploying)
	const proto = structuredClone(config);
	proto.protocols.ftp.credentials.server = 'X';
	assert.deepEqual(deploys([TARGETS_FILE], JSON.stringify(proto)), config.order);
	assert.deepEqual(deploys([TARGETS_FILE], null), config.order);
});

test('package closure: unknown packages or unreadable package.json → every shared dir', () => {
	const pkgs = { 'app/package.json': { dependencies: { '@scutoons/a': '*' } }, 'shared/a/package.json': { name: '@scutoons/a', dependencies: { '@scutoons/b': '*' } }, 'shared/b/package.json': { name: '@scutoons/b' }, 'shared/c/package.json': { name: '@scutoons/c' } };
	const read = (p) => pkgs[p] ?? null;
	const shared = ['shared/a', 'shared/b', 'shared/c'];
	assert.deepEqual(packageClosure(['app'], read, shared), ['shared/a', 'shared/b']);
	pkgs['app/package.json'].dependencies['@scutoons/new'] = '*';
	assert.deepEqual(packageClosure(['app'], read, shared), shared);
	assert.deepEqual(packageClosure(['missing'], read, shared), shared);
});

test('every app package.json dependency on @scutoons/* is a shared dir the app watches', () => {
	for (const name of config.order) {
		const t = target(name);
		for (const dir of t.watchPackages ?? []) {
			const pkg = JSON.parse(readFileSync(new URL(`../../${dir}/package.json`, import.meta.url), 'utf8'));
			for (const dep of Object.keys({ ...pkg.dependencies, ...pkg.devDependencies }).filter((d) => d.startsWith('@scutoons/'))) {
				const sharedDir = `shared/${dep.slice('@scutoons/'.length)}`;
				assert.ok(relevantFiles(t, [`${sharedDir}/package.json`]).length, `${name} misses ${sharedDir}`);
			}
		}
	}
});
