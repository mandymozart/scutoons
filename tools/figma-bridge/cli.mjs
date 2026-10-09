// SCUTOONS Figma bridge – command line (talks to the relay on 127.0.0.1; the relay talks to the open plugin).
//   node tools/figma-bridge/cli.mjs status
//   node tools/figma-bridge/cli.mjs ping
//   node tools/figma-bridge/cli.mjs pages
//   node tools/figma-bridge/cli.mjs read [--page "S1C3" | --node 555:2354] [--depth 2]
//   node tools/figma-bridge/cli.mjs export --node 555:2354 [--format png|svg] [--scale 1] --to out.png
//   node tools/figma-bridge/cli.mjs draft storyboards/_out/s1c3/figma/scene-25.spec.json [more specs …]

import { readFileSync, writeFileSync } from 'node:fs';

export async function bridge(cmd, args = {}, port = Number(process.env.SCUTOONS_FIGMA_PORT ?? 3056)) {
	const res = await fetch(`http://127.0.0.1:${port}/cmd`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ cmd, args }) }).catch(() => null);
	if (!res) return { ok: false, error: 'the relay is not running – node tools/figma-bridge/relay.mjs' };
	return res.json();
}

async function main() {
	const argv = process.argv.slice(2);
	const cmd = argv.shift();
	const opt = (f, d) => { const i = argv.indexOf(f); return i >= 0 ? argv.splice(i, 2)[1] : d; };
	const port = Number(process.env.SCUTOONS_FIGMA_PORT ?? 3056);
	if (cmd === 'status') {
		const r = await fetch(`http://127.0.0.1:${port}/status`).then((x) => x.json()).catch(() => null);
		console.log(r ? JSON.stringify(r) : 'relay not running');
		return;
	}
	let out;
	if (cmd === 'ping' || cmd === 'pages') out = await bridge(cmd, {}, port);
	else if (cmd === 'read') out = await bridge('read', { page: opt('--page'), nodeId: opt('--node'), depth: Number(opt('--depth', 2)) }, port);
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
			console.log(`${file}: ${r.ok ? JSON.stringify(r.result) : `ERROR ${r.error}`}`);
			if (!r.ok) process.exitCode = 1;
		}
		return;
	} else {
		console.log('commands: status · ping · pages · read · export · draft (see the head of cli.mjs)');
		process.exitCode = 2;
		return;
	}
	console.log(JSON.stringify(out.ok ? out.result : { error: out.error }, null, 2));
	if (!out.ok) process.exitCode = 1;
}

if (process.argv[1]?.endsWith('cli.mjs')) main();
