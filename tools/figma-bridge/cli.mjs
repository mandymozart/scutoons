// SCUTOONS Figma bridge – command line (talks to the relay on 127.0.0.1; the relay talks to the open plugin).
//   node tools/figma-bridge/cli.mjs status       the relay's /status: plugin, stale, busy, queue, code version, …
//   node tools/figma-bridge/cli.mjs preflight    what a long run checks first (relay – started if not –, plugin, its
//                                                code version = code.js, a ping, free RAM); waits like the runners do
//   node tools/figma-bridge/cli.mjs ping
//   node tools/figma-bridge/cli.mjs pages
//   node tools/figma-bridge/cli.mjs read [--page "S1C3" | --node 555:2354] [--depth 2]
//   node tools/figma-bridge/cli.mjs export --node 555:2354 [--format png|svg] [--scale 1] --to out.png
//   node tools/figma-bridge/cli.mjs draft storyboards/_out/s1c3/figma/scene-25.spec.json [more specs …]
// One try per command (node:http, the per-command timeouts of scripts/storyboard/lib.mjs); long runs use figma() from
// there, which waits for the plugin and retries.

import { readFileSync, writeFileSync } from 'node:fs';
import { figmaOnce, figmaPreflight, figmaStatus } from '../../scripts/storyboard/lib.mjs';

/** one command → { ok: true, result } | { ok: false, code, error } (scripts/vectors-survey.mjs uses it too) */
export async function bridge(cmd, args = {}, port = Number(process.env.SCUTOONS_FIGMA_PORT ?? 3056), opts = {}) {
	return figmaOnce(cmd, args, { port, client: 'cli.mjs', ...opts });
}

async function main() {
	const argv = process.argv.slice(2);
	const cmd = argv.shift();
	const opt = (f, d) => { const i = argv.indexOf(f); return i >= 0 ? argv.splice(i, 2)[1] : d; };
	const port = Number(process.env.SCUTOONS_FIGMA_PORT ?? 3056);
	if (cmd === 'status') {
		const r = await figmaStatus(port);
		console.log(r ? JSON.stringify(r, null, 2) : `relay not running on :${port} – node tools/figma-bridge/supervise.mjs --detach`);
		if (!r) process.exitCode = 1;
		return;
	}
	if (cmd === 'preflight') {
		try { console.log(JSON.stringify(await figmaPreflight({ port }), null, 2)); } catch (e) { console.error(`${e.code ?? 'error'}: ${e.message}`); process.exitCode = 1; }
		return;
	}
	let out;
	if (cmd === 'ping' || cmd === 'pages') out = await bridge(cmd, {}, port);
	else if (cmd === 'read') out = await bridge('read', { page: opt('--page'), nodeId: opt('--node'), depth: Number(opt('--depth', 2)) }, port, { timeout: 240_000 });
	else if (cmd === 'export') {
		const to = opt('--to');
		out = await bridge('export', { nodeId: opt('--node'), page: opt('--page'), format: opt('--format', 'png'), scale: Number(opt('--scale', 1)) }, port);
		if (out.ok && to) {
			writeFileSync(to, Buffer.from(out.result.base64, 'base64'));
			out = { ok: true, result: { ...out.result, base64: undefined, to } };
		}
	} else if (cmd === 'draft') {
		for (const file of argv) {
			const spec = JSON.parse(readFileSync(file, 'utf8'));
			const r = await bridge('draft', { spec }, port);
			console.log(`${file}: ${r.ok ? JSON.stringify(r.result) : `ERROR ${r.code}: ${r.error}`}`);
			if (!r.ok) process.exitCode = 1;
		}
		return;
	} else {
		console.log('commands: status · preflight · ping · pages · read · export · draft (see the head of cli.mjs)');
		process.exitCode = 2;
		return;
	}
	console.log(JSON.stringify(out.ok ? out.result : { code: out.code, error: out.error }, null, 2));
	if (!out.ok) process.exitCode = 1;
}

if (process.argv[1]?.endsWith('cli.mjs')) main();
