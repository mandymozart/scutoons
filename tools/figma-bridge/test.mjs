// Self-test of the bridge without Figma: starts the relay on a spare port, connects a stand-in plugin over a real
// WebSocket (Node's global WebSocket), and sends commands through the CLI's HTTP path – checks the handshake, framing
// (small and > 64 KiB messages), the round trip and the error paths. The stand-in also runs applyDraft against a tiny
// fake of the Plugin API, so the draft code is exercised too.
//   node tools/figma-bridge/test.mjs

import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { startRelay } from './relay.mjs';
import { bridge } from './cli.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const PORT = 3999;
const quiet = [];
const relay = await startRelay({ port: PORT, log: (m) => quiet.push(m) });

// before a plugin: a clear error
let r = await bridge('ping', {}, PORT);
assert.equal(r.ok, false);
assert.match(r.error, /no plugin connected/);

// the fake Plugin API (just enough for applyDraft)
function fakeFigma() {
	let nid = 0;
	const node = (type, extra = {}) => {
		const n = { id: `0:${++nid}`, type, name: '', x: 0, y: 0, width: 0, height: 0, children: [], parent: null, locked: false, visible: true, ...extra };
		n.appendChild = (c) => { if (c.parent) c.parent.children = c.parent.children.filter((k) => k !== c); c.parent = n; n.children.push(c); };
		n.remove = () => { if (n.parent) n.parent.children = n.parent.children.filter((k) => k !== n); };
		n.resize = (w, h) => { n.width = w; n.height = h; };
		n.rescale = (s) => { n.width *= s; n.height *= s; };
		return n;
	};
	const root = node('DOCUMENT', { name: 'Cat-Pope' });
	let current = null;
	// like Figma: a new node lands on the current page
	const onPage = (n) => { current?.appendChild(n); return n; };
	return {
		root,
		createPage: () => { const p = node('PAGE'); root.appendChild(p); return p; },
		setCurrentPageAsync: async (p) => { current = p; },
		loadFontAsync: async () => {},
		createFrame: () => onPage(node('FRAME')),
		createRectangle: () => onPage(node('RECTANGLE')),
		createText: () => { const t = onPage(node('TEXT')); t.height = 16; return t; },
		createNodeFromSvg: (svg) => { const m = /width="(\d+)" height="(\d+)"/.exec(svg); return onPage(node('FRAME', { width: +m[1], height: +m[2] })); },
		group: (nodes, parent) => { const g = node('GROUP'); parent.appendChild(g); nodes.forEach((n) => g.appendChild(n)); return g; }
	};
}
const code = readFileSync(resolve(here, 'plugin/code.js'), 'utf8');
const applierSrc = /\/\/ <applyDraft>[^\n]*\n([\s\S]*?)\/\/ <\/applyDraft>/.exec(code)[1];
const figma = fakeFigma();
const applyDraft = new Function('figma', `${applierSrc}\nreturn applyDraft;`)(figma);

// the stand-in plugin
const ws = new WebSocket(`ws://127.0.0.1:${PORT}/plugin`);
await new Promise((ok, fail) => { ws.onopen = ok; ws.onerror = fail; });
ws.onmessage = async (e) => {
	const msg = JSON.parse(e.data);
	try {
		let result;
		if (msg.cmd === 'ping') result = { file: 'Cat-Pope', page: 'S1C3' };
		else if (msg.cmd === 'echo') result = msg.args;
		else if (msg.cmd === 'draft') result = await applyDraft(msg.args.spec);
		else throw new Error(`unknown command ${msg.cmd}`);
		ws.send(JSON.stringify({ id: msg.id, ok: true, result }));
	} catch (err) {
		ws.send(JSON.stringify({ id: msg.id, ok: false, error: err.message }));
	}
};
await new Promise((ok) => setTimeout(ok, 100));

r = await bridge('ping', {}, PORT);
assert.deepEqual(r, { id: r.id, ok: true, result: { file: 'Cat-Pope', page: 'S1C3' } });
const big = 'x'.repeat(200_000); // > 64 KiB both ways (16-bit and 64-bit frame lengths)
r = await bridge('echo', { big }, PORT);
assert.equal(r.result.big.length, 200_000);
r = await bridge('nope', {}, PORT);
assert.equal(r.ok, false);
assert.match(r.error, /unknown command/);

// a real spec from the storyboards, if one was generated
const specFile = resolve(here, '../../storyboards/_out/s1c3/figma/scene-25.spec.json');
let spec;
try { spec = JSON.parse(readFileSync(specFile, 'utf8')); } catch { spec = null; }
if (spec) {
	r = await bridge('draft', { spec }, PORT);
	assert.equal(r.ok, true, r.error);
	assert.equal(r.result.shots, spec.shots.length);
	const again = await bridge('draft', { spec }, PORT); // idempotent: replaced by id, not doubled
	assert.equal(again.result.shots, spec.shots.length);
	assert.equal(again.result.texts, spec.shots.reduce((a, s) => a + s.texts.length, 0));
	const page = figma.root.children[0];
	const frame = page.children.find((n) => n.name === spec.frame.name);
	assert.deepEqual(frame.children.map((c) => c.name), ['storyboard', 'guides', 'text', 'layout']);
	assert.equal(frame.children[0].locked, true);
	assert.equal(frame.children[1].visible, false);
	assert.equal(page.children.filter((n) => n.name.endsWith('· rail')).length, 1);
}

ws.close();
await relay.close();
console.log(`bridge self-test ok${spec ? ` (draft ${spec.frame.name}: ${spec.shots.length} shots, idempotent)` : ' (no spec – run figma-draft.mjs for the draft check)'}`);
