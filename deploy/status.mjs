#!/usr/bin/env node
// What is live: each target's commit (its status URL, deploy/targets.json) next to main.
//   npm run deploy:status [-- --ref origin/main]
import { execFileSync } from 'node:child_process';
import { git, knownCommit, liveSha, ordered, watched } from './live.mjs';

const args = process.argv.slice(2);
const ref = args.includes('--ref') ? args[args.indexOf('--ref') + 1] : 'origin/main';
let main;
try {
	execFileSync('git', ['fetch', '--quiet', 'origin', 'main'], { stdio: 'ignore', env: { ...process.env, GIT_TERMINAL_PROMPT: '0' } });
} catch {
	// offline / no credentials: compare with the local ref
}
try {
	main = git('rev-parse', ref);
} catch {
	main = git('rev-parse', 'main');
}

console.log(`main = ${main.slice(0, 7)} (${ref})\n`);
let behind = 0;
for (const target of ordered()) {
	const live = await liveSha(target);
	let state;
	if (!live) state = '✗ unknown (not reachable / no version.json)';
	else if (live === main || main.startsWith(live)) state = '✓ = main';
	else if (!knownCommit(live)) state = '✗ commit not in this clone';
	else {
		const relevant = git('diff', '--name-only', live, main).split('\n').filter((f) => f && watched(target, f));
		state = relevant.length ? `✗ behind main (${relevant.length} changed files)` : '✓ no relevant change since';
	}
	if (state.startsWith('✗')) behind++;
	console.log(`${target.name.padEnd(8)} ${(live ?? '–').slice(0, 7).padEnd(8)} ${state}   ${target.status ?? ''}`);
}
process.exit(behind ? 1 : 0);
