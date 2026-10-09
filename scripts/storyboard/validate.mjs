// Validate storyboards (workflows/scenes/storyboard.md): structure errors fail, rhythm / safe-format findings warn.
//   node scripts/storyboard/validate.mjs storyboards/catpope/s1c3 [more files / folders]

import { TARGET, fmtMin } from './lib.mjs';
import { loadStoryboards, summarize, validate } from './storyboard.mjs';

const paths = process.argv.slice(2);
if (!paths.length) {
	console.error('usage: validate.mjs <storyboard file or folder> …');
	process.exit(2);
}
let errors = 0;
const all = [];
for (const p of paths) {
	const list = loadStoryboards(p);
	all.push(...list);
	for (const { file, sb } of list) {
		const r = validate(sb, file);
		for (const e of r.errors) console.log(`ERROR   ${e}`);
		for (const w of r.warnings) console.log(`warning ${w}`);
		errors += r.errors.length;
	}
}
const { scenes, total } = summarize(all);
console.log('\nscene  shots  screens  words  careful');
for (const s of scenes) console.log(`${String(s.scene).padStart(5)}  ${String(s.shots).padStart(5)}  ${s.screens.toFixed(1).padStart(7)}  ${String(s.words).padStart(5)}  ${fmtMin(s.seconds).padStart(8)}  ${s.heading ?? ''}`);
console.log(`total  ${String(total.shots).padStart(5)}  ${total.screens.toFixed(1).padStart(7)}  ${String(total.words).padStart(5)}  ${fmtMin(total.seconds).padStart(8)}   (target ${TARGET.screens.join('–')} screens, ${TARGET.minutes.join('–')} min)`);
console.log(errors ? `\n${errors} error(s)` : '\nno errors');
process.exit(errors ? 1 : 0);
