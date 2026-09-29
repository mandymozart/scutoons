// @scutoons/tokens check: every DESIGN.md Part 1 token is defined, every url() in the CSS resolves to a file,
// every font file is referenced, and base.css only uses defined tokens.
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const read = (file) => readFileSync(join(root, file), 'utf8');
const errors = [];

const required = [
	'ink', 'paper', 'primary-cyan', 'acid-purple', 'acid-green', 'acid-red', 'border-muted', 'border-subtle',
	'border-hover', 'surface-hover', 'surface-panel', 'surface-tint-cyan', 'text-muted',
	'space-025', 'space-05', 'space-075', 'space-1', 'space-15', 'space-3',
	'radius-none', 'radius-sm', 'radius-full', 'border-width-hairline', 'border-width-regular',
	'font-mono', 'font-lettering', 'type-body-size', 'type-body-line', 'type-button-size', 'type-button-line',
	'type-label-size', 'type-label-line', 'type-caption-size', 'type-caption-line', 'type-nav-micro-size',
	'type-nav-micro-line', 'root-size-small', 'root-size', 'root-size-large'
];

const tokens = read('tokens.css');
const defined = new Set([...tokens.matchAll(/--([a-z0-9-]+)\s*:/g)].map((m) => m[1]));
for (const name of required) if (!defined.has(name)) errors.push(`tokens.css: --${name} is missing`);

for (const file of ['tokens.css', 'base.css', 'fonts.css']) {
	for (const [, name] of read(file).matchAll(/var\(--([a-z0-9-]+)/g)) {
		if (!defined.has(name)) errors.push(`${file}: var(--${name}) is not defined in tokens.css`);
	}
}

const fonts = read('fonts.css');
const referenced = new Set();
for (const [, url] of fonts.matchAll(/url\(([^)]+)\)/g)) {
	const path = url.replace(/['"]/g, '');
	referenced.add(path.replace(/^\.\/fonts\//, ''));
	if (!existsSync(join(root, path))) errors.push(`fonts.css: ${path} does not exist`);
}
for (const file of readdirSync(join(root, 'fonts'))) {
	if (file.endsWith('.woff2') && !referenced.has(file)) errors.push(`fonts/${file} is not used in fonts.css`);
}

const index = read('index.css');
for (const file of ['tokens.css', 'fonts.css', 'base.css']) {
	if (!index.includes(`'./${file}'`)) errors.push(`index.css does not import ${file}`);
}

if (errors.length) {
	console.error(errors.join('\n'));
	process.exit(1);
}
console.log(`@scutoons/tokens: ${defined.size} tokens, ${referenced.size} font files – ok`);
