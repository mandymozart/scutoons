// The SCUTOONS MCP's figma_* tools (tools/mcp/src/figma.ts) end to end without Figma: the relay on a spare port, a
// stand-in plugin over a real WebSocket, the tools' handlers called as the MCP server would.
//   node --experimental-transform-types --no-warnings tools/figma-bridge/test-mcp.mjs

import assert from 'node:assert/strict';
import { writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { startRelay } from './relay.mjs';
import { registerFigmaTools } from '../mcp/src/figma.ts';

const PORT = 3998;
const tools = {};
registerFigmaTools({ registerTool: (name, def, handler) => (tools[name] = { def, handler }) }, { port: PORT });
assert.deepEqual(Object.keys(tools).sort(), ['figma_place_draft', 'figma_read', 'figma_status', 'figma_view']);

// no relay → the setup hint
let r = await tools.figma_status.handler({});
assert.equal(r.isError, true);
assert.match(r.content[0].text, /relay\.mjs/);

const relay = await startRelay({ port: PORT, log: () => {} });
r = await tools.figma_status.handler({});
assert.equal(r.isError, true, 'relay without plugin');
assert.match(r.content[0].text, /SCUTOONS Bridge started/);

const PNG = Buffer.from('89504e470d0a1a0a', 'hex').toString('base64');
const seen = [];
const ws = new WebSocket(`ws://127.0.0.1:${PORT}/plugin`);
await new Promise((ok, fail) => { ws.onopen = ok; ws.onerror = fail; });
ws.onmessage = (e) => {
	const m = JSON.parse(e.data);
	seen.push([m.cmd, m.args]);
	const answer = {
		ping: { file: 'Cat-Pope', page: 'S1C3', user: 'Mandy Mozart' },
		pages: [{ id: '0:1', name: 'S1C3' }],
		read: { id: m.args.nodeId ?? '0:1', name: 'S1C3', type: 'PAGE', childCount: 0 },
		export: { name: 'photo', format: 'PNG', bytes: 8, base64: PNG },
		draft: { page: '9:1', frame: '9:2', shots: m.args.spec?.shots?.length ?? 0, texts: 0, layout: 0 }
	}[m.cmd];
	ws.send(JSON.stringify(answer ? { id: m.id, ok: true, result: answer } : { id: m.id, ok: false, error: 'unknown' }));
};
await new Promise((ok) => setTimeout(ok, 100));

r = await tools.figma_status.handler({});
assert.equal(r.isError, undefined);
assert.deepEqual(JSON.parse(r.content[0].text).pages, [{ id: '0:1', name: 'S1C3' }]);

r = await tools.figma_read.handler({ node: '555-2354', depth: 1 });
assert.equal(JSON.parse(r.content[0].text).id, '555:2354', 'URL form 555-2354 → 555:2354');

r = await tools.figma_view.handler({ node: '1:2' });
assert.equal(r.content[1].type, 'image');
assert.equal(r.content[1].mimeType, 'image/png');
assert.deepEqual(seen.find(([c]) => c === 'export')[1], { nodeId: '1:2', format: 'PNG', scale: 0.5 });

const spec = join(tmpdir(), 'scutoons-test-spec.json');
writeFileSync(spec, JSON.stringify({ page: 'T', frame: { name: 'f', x: 0, w: 390, h: 844 }, shots: [{ id: 'sb-1-1', svg: '<svg/>', y: 0 }] }));
r = await tools.figma_place_draft.handler({ spec_file: spec });
assert.equal(JSON.parse(r.content[0].text).shots, 1);

ws.close();
await relay.close();
console.log('MCP figma tools ok (status, read, view, place_draft; setup hints without relay / plugin)');
