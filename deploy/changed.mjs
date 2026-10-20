#!/usr/bin/env node
// Which targets a deploy run has to upload: for each target (deploy order), the files changed between the commit it
// is serving right now (deploy/live.mjs) and HEAD, filtered by `relevantFiles` (deploy/live.mjs:
// its `watch` + the shared packages it imports, minus `watchIgnore`; deploy/targets.json only when its block changed).
// Comparing with what is LIVE (not with the previous push) means a skipped or failed run is caught by the next one.
// A target whose live commit is unknown (unreachable, first deploy) is deployed.
//   node deploy/changed.mjs [--apps changed|all|<name,name>] [--head <sha>] [--github]
//   → prints the plan; --github writes `apps=[…]` (deploy order) to GITHUB_OUTPUT and the plan to the run summary.
// `all` = every target in `order`; explicit names may also be held-back targets (embassy).
import { appendFileSync } from 'node:fs';
import { config, configAt, git, knownCommit, liveSha, ordered, relevantFiles } from './live.mjs';

const args = process.argv.slice(2);
const choice = (args.includes('--apps') ? args[args.indexOf('--apps') + 1] : '') || 'changed';
const all = choice === 'all';
const named = all || choice === 'changed' ? null : choice.split(',').map((a) => a.trim()).filter(Boolean);
for (const name of named ?? []) {
	if (!config.targets[name]) {
		console.error(`unknown target "${name}" (deploy/targets.json: ${Object.keys(config.targets).join(', ')})`);
		process.exit(1);
	}
}
const head = args.includes('--head') ? args[args.indexOf('--head') + 1] : git('rev-parse', 'HEAD');

const apps = [];
const lines = [];
const targets = named ? [...config.order, ...Object.keys(config.targets)].filter((n, i, list) => named.includes(n) && list.indexOf(n) === i).map((name) => ({ name, ...config.targets[name] })) : ordered();
for (const target of targets) {
	if (all || named) {
		apps.push(target.name);
		lines.push(`${target.name.padEnd(8)} deploy (requested)`);
		continue;
	}
	const live = await liveSha(target);
	if (!live || !knownCommit(live)) {
		apps.push(target.name);
		lines.push(`${target.name.padEnd(8)} deploy (live commit ${live ?? 'unknown'})`);
		continue;
	}
	const files = git('diff', '--name-only', live, head).split('\n').filter(Boolean);
	const hits = relevantFiles(target, files, { oldConfig: configAt(live) });
	if (hits.length) {
		apps.push(target.name);
		lines.push(`${target.name.padEnd(8)} deploy (${hits.length} changed since ${live.slice(0, 7)}, e.g. ${hits[0]})`);
	} else lines.push(`${target.name.padEnd(8)} up to date (${live.slice(0, 7)})`);
}

console.log(lines.join('\n'));
if (args.includes('--github') && process.env.GITHUB_OUTPUT) {
	appendFileSync(process.env.GITHUB_OUTPUT, `apps=${JSON.stringify(apps)}\n`);
	if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, `### Deploy plan\n\n\`\`\`\n${lines.join('\n')}\n\`\`\`\n`);
}
