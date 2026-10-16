// Writes the shared boot screen (shared/boot/boot.css + boot.js) into each app's src/app.html, between the markers
// <!-- sc-boot:style --> … <!-- /sc-boot:style --> (head) and <!-- sc-boot:body --> … <!-- /sc-boot:body --> (body).
// The boot screen can't be a component: it shows before the app's code is there – so one source, written inline.
//   npm run boot:sync          write
//   npm run boot:sync -- --check   fail when an app.html is out of date (part of `npm run check`)
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
// per app: the expected app code before the first draw (uncompressed, only the line's scale – measured 2026-10-16) and
// the skeleton (the reader's page shapes; creator / admin: the logo and the line alone)
const APPS = {
	viewer: { expect: 900000, skeleton: 'reader' },
	creator: { expect: 1300000 },
	admin: { expect: 1300000 }
};

const css = readFileSync(resolve(root, 'shared/boot/boot.css'), 'utf8').trim();
const js = readFileSync(resolve(root, 'shared/boot/boot.js'), 'utf8').trim();
const indent = (text, tabs) => text.split('\n').map((l) => (l ? '\t'.repeat(tabs) + l : l)).join('\n');
const NOTE = 'generated from shared/boot by `npm run boot:sync` – edit there';

function block(app) {
	const { expect, skeleton } = APPS[app];
	const style = `<!-- sc-boot:style (${NOTE}) -->\n\t\t<style>\n${indent(css, 3)}\n\t\t</style>\n\t\t<!-- /sc-boot:style -->`;
	const body = [
		`<!-- sc-boot:body (${NOTE}) -->`,
		`<div id="sc-boot" data-expect="${expect}"${skeleton ? ` data-skeleton="${skeleton}"` : ''}>`,
		'\t<div class="mid">',
		'\t\t<img src="%sveltekit.assets%/SCUTOONS-logo-animated.svg" alt="" />',
		'\t\t<span class="text"></span>',
		'\t</div>',
		'\t<span class="bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow="0"><span class="fill"></span></span>',
		'</div>',
		'<div id="sc-app" style="display: contents">%sveltekit.body%</div>',
		'<script>',
		indent(js, 1),
		'</script>',
		'<!-- /sc-boot:body -->'
	]
		.map((l, i) => (i ? '\t\t' + l : l))
		.join('\n');
	return { style, body };
}

const replaceBetween = (html, name, content, file) => {
	const re = new RegExp(`<!-- ${name}[^>]*-->[\\s\\S]*?<!-- /${name} -->`);
	if (!re.test(html)) throw new Error(`${file}: no <!-- ${name} --> … <!-- /${name} --> markers`);
	return html.replace(re, () => content);
};

const check = process.argv.includes('--check');
let stale = 0;
for (const app of Object.keys(APPS)) {
	const file = resolve(root, app, 'src/app.html');
	const html = readFileSync(file, 'utf8');
	const { style, body } = block(app);
	const next = replaceBetween(replaceBetween(html, 'sc-boot:style', style, file), 'sc-boot:body', body, file);
	if (next === html) continue;
	if (check) {
		console.error(`${app}/src/app.html: the boot screen is out of date – npm run boot:sync`);
		stale++;
	} else {
		writeFileSync(file, next);
		console.log(`${app}/src/app.html: boot screen written`);
	}
}
if (stale) process.exit(1);
