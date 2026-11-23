// Writes the projector's view of the test experience's poster scene into the contract's shared vectors as the `xr`
// view (run once when the projector changes on purpose; network.test.ts checks they agree). The other views keep their
// hand-written layout: the xr view is the views list's last entry, an older one is cut first. Test-only.
import { readFileSync, writeFileSync } from 'node:fs';
import { projectXr } from '../src/network.ts';
import { doc } from './network-fixture.ts';

const file = new URL('../../../server/schema/api/spore-network.vectors.json', import.meta.url);
const text = readFileSync(file, 'utf8');
const view = projectXr(doc, 'experience:demo/scene:poster');
const mark = '\n\t\t{\n\t\t\t"scope": "xr"';
const close = text.lastIndexOf('\n\t]');
const base = text.includes(mark) ? `${text.slice(0, text.indexOf(mark) - 1)}${text.slice(close)}` : text;
const end = base.lastIndexOf('\n\t]');
const entry = JSON.stringify(view, null, '\t')
	.split('\n')
	.map((line) => `\t\t${line}`)
	.join('\n');
writeFileSync(file, `${base.slice(0, end)},\n${entry}${base.slice(end)}`);
JSON.parse(readFileSync(file, 'utf8'));
console.log(`xr view: ${view.spores.length} spores, ${view.threads.length} threads, ${view.stubs.length} stubs, ${view.problems.length} problems`);
