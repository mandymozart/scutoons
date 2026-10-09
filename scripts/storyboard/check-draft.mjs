// The creator's own checks (validateNeutral, shared/content) on a compiled chapter draft – what save_draft would run.
//   node --experimental-transform-types --no-warnings scripts/storyboard/check-draft.mjs storyboards/_out/s1c3/chapter.draft.json [chapter:catpope:3]
// Missing translations are counted, not listed (they come with the translation step).

import { readFileSync } from 'node:fs';
import { validateNeutral } from '../../shared/content/src/validate.ts';

const [file, key = 'chapter'] = process.argv.slice(2);
const doc = JSON.parse(readFileSync(file, 'utf8'));
const problems = validateNeutral('chapter', doc, { source: 'de', languages: ['de', 'en'] }, { file: key });
const missing = problems.filter((p) => /^missing \w+ text/.test(p.message));
const rest = problems.filter((p) => !missing.includes(p));
for (const p of rest) console.log(`${p.level.padEnd(7)} ${p.locale ?? ''} ${p.path.join('.')} – ${p.message}`);
console.log(`\n${rest.filter((p) => p.level === 'error').length} errors · ${rest.filter((p) => p.level === 'warning').length} warnings · ${missing.length} missing translations`);
process.exit(rest.some((p) => p.level === 'error') ? 1 : 0);
