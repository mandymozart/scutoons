import type { XrHandlers, XrRun } from '@scutoons/xr-engine/reader';
import type { XrGameConfiguration, XrPlugin } from '@scutoons/content';
import { asset } from '../utils/cdn.ts';
import { withPlugins } from './xr-session.ts';

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
 * experience in `container`. The document's plugins are imported from blob URLs, which live as long as the scene.
 */
export async function openXr(
	container: HTMLElement,
	experience: { assets: string; configuration: XrGameConfiguration; plugins: readonly XrPlugin[] },
	scene: string,
	language: () => string,
	handlers: XrHandlers
): Promise<XrRun> {
	const engine = await import('@scutoons/xr-engine/reader');
	const urls: string[] = [];
	const release = () => urls.splice(0).forEach((url) => URL.revokeObjectURL(url));
	const configuration = withPlugins(experience.configuration, experience.plugins, (code) => {
		const url = URL.createObjectURL(new Blob([code], { type: 'text/javascript' }));
		urls.push(url);
		return url;
	});

	// the configuration's /assets/content/… paths lie in the experience's own CDN folder
	const assetsBase = asset(experience.assets).replace(/\/+$/, '');
	const opened = await engine.openXr(container, configuration, scene, { xr8Base: XR8_BASE, assetsBase, language, handlers }).catch((e) => {
		release();
		throw e;
	});
	if ('problems' in opened) {
		release();
		throw new XrContentError(opened.problems);
	}
	const { run } = opened;
	return { ...run, dispose: () => run.dispose().finally(release) };
}
