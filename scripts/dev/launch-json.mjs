#!/usr/bin/env node
// Writes .claude/launch.json (local, gitignored: the Claude desktop app's preview servers) from shared/dev-ports.json,
// so its ports never drift from the single source of truth. The owner's stack still starts with `npm run dev`.
//   npm run dev:launch-json
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { PORTS } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

/** service → [executable, args] (cwd = the repository root) */
const COMMANDS = {
	viewer: ['npm', ['run', 'dev', '-w', 'viewer']],
	cdn: ['node', ['scripts/dev/media-serve.mjs']],
	api: ['node', ['server/bin/dev-serve.mjs']],
	qr: ['php', ['scripts/dev/php-serve.php', 'qr', 'qr/public', 'qr/public/router.php']],
	creator: ['npm', ['run', 'dev', '-w', 'creator']],
	admin: ['npm', ['run', 'dev', '-w', 'admin']],
	shop: ['npm', ['run', 'dev', '-w', 'shop']],
	embassy: ['npm', ['run', 'dev', '-w', 'embassy']],
	studio: ['npm', ['run', 'dev', '-w', 'workflows/storyboard-studio']], // the storyboard studio (a local workflow tool)
	// docker/compose.yml (foreground `up`, so the preview server owns it); mariadb is no web page – not listed
	adminer: ['docker', ['compose', '-f', 'docker/compose.yml', '--profile', 'tools', 'up', 'adminer']]
};
const NOT_WEB = new Set(['mariadb']);

const configurations = Object.entries(PORTS).filter(([name]) => !NOT_WEB.has(name)).map(([name, port]) => {
	if (!COMMANDS[name]) throw new Error(`shared/dev-ports.json: no command for "${name}" in scripts/dev/launch-json.mjs`);
	const [runtimeExecutable, runtimeArgs] = COMMANDS[name];
	return { name, runtimeExecutable, runtimeArgs, port };
});

const file = resolve(ROOT, '.claude/launch.json');
mkdirSync(dirname(file), { recursive: true });
writeFileSync(file, JSON.stringify({ version: '0.0.1', configurations }, null, 2) + '\n');
console.log(`${file}: ${configurations.map((c) => `${c.name} ${c.port}`).join(', ')}`);
