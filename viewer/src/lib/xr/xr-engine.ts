import type { XrHandlers, XrRun } from '@scutoons/xr-engine/reader';
import type { XrGameConfiguration, XrPlugin } from '@scutoons/content';
import { asset } from '../utils/cdn.ts';

export type { XrRun };

/** where the build serves the image tracking's files (vite/xr8-engine.ts) */
const XR8_BASE = '/xr8/';

/** the experience's configuration is not one this engine can read; `problems`: one line each, for the console */
export class XrContentError extends Error {
	constructor(readonly problems: string[]) {
		super(problems[0] ?? 'the experience could not be read');
		this.name = 'XrContentError';
	}
}

/**
 * Loads OSCT's engine (shared/xr-engine: three.js and the scene, a chunk of its own) and builds one scene of the
 * experience in `container`.
 */
export async function openXr(
	container: HTMLElement,
	experience: { assets: string; configuration: XrGameConfiguration; plugins: readonly XrPlugin[] },
	scene: string,
	language: () => string,
	handlers: XrHandlers
): Promise<XrRun> {
	const engine = await import('@scutoons/xr-engine/reader');
	// the configuration's /assets/content/… paths lie in the experience's own CDN folder
	const assetsBase = asset(experience.assets).replace(/\/+$/, '');
	const opened = await engine.openXr(container, experience.configuration, experience.plugins, scene, { xr8Base: XR8_BASE, assetsBase, language, handlers });
	if ('problems' in opened) throw new XrContentError(opened.problems);
	return opened.run;
}
