// The resolve hook of register.mjs: aliases to the copy's folders, and a missing file tried as `.ts` / `index.ts`.
import { existsSync, statSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = new URL('../', import.meta.url);
const ALIASES = [
	['@engine/', new URL('osct/engine/src/', root)],
	['@shared/', new URL('osct/shared/', root)],
	['#lib/', new URL('osct/editor/src/lib/', root)]
];

function fileOf(url) {
	const path = fileURLToPath(url);
	for (const candidate of [path, path.replace(/\.js$/, '.ts'), `${path}.ts`, `${path}/index.ts`]) if (existsSync(candidate) && statSync(candidate).isFile()) return pathToFileURL(candidate).href;
	return null;
}

export async function resolve(specifier, context, next) {
	for (const [prefix, base] of ALIASES) {
		if (specifier.startsWith(prefix)) {
			const found = fileOf(new URL(specifier.slice(prefix.length), base));
			if (found) return { url: found, shortCircuit: true };
		}
	}
	if ((specifier.startsWith('./') || specifier.startsWith('../')) && context.parentURL?.startsWith('file:')) {
		const target = new URL(specifier, context.parentURL);
		if (!existsSync(fileURLToPath(target)) || !statSync(fileURLToPath(target)).isFile()) {
			const found = fileOf(target);
			if (found) return { url: found, shortCircuit: true };
		}
	}
	return next(specifier, context);
}
