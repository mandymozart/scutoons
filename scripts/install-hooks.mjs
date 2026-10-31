// Installs the repo's git hooks (scripts/git-hooks/) into .git/hooks – today the pre-push: Git LFS + the local deck CI
// (scripts/deck-ci.mjs). Run once per checkout: npm run hooks
import { copyFileSync, chmodSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const hooks = resolve(root, execFileSync('git', ['rev-parse', '--git-path', 'hooks'], { cwd: root, encoding: 'utf8' }).trim());
for (const name of ['pre-push']) {
	copyFileSync(resolve(root, 'scripts/git-hooks', name), resolve(hooks, name));
	chmodSync(resolve(hooks, name), 0o755);
	console.log(`installed ${name} → ${resolve(hooks, name)}`);
}
