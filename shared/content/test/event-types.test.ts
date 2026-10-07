// Every VN event type the validator knows is also in the stored (language-neutral) documents' schema – the schema is
// what the API checks a save against. Two types were missing there once (fx, then giveItem / takeItem: the creator's
// save was refused with "doesn't fit its type"):
//   npm test -w shared/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const root = new URL('../', import.meta.url);
const validate = readFileSync(new URL('src/validate.ts', root), 'utf8');
const known = JSON.parse(validate.match(/const EVENT_TYPES = (\[[^\]]*\])/)![1].replace(/'/g, '"')) as string[];

for (const doc of ['chapter-doc', 'group-doc']) {
	test(`${doc}.schema.json has every event type`, () => {
		const schema = readFileSync(new URL(`schema/${doc}.schema.json`, root), 'utf8');
		const missing = known.filter((type) => !schema.includes(`"const": "${type}"`));
		assert.deepEqual(missing, [], `missing in ${doc}: ${missing.join(', ')} – add it to VNEventDoc (neutral.ts) and run npm run schema`);
	});
}
