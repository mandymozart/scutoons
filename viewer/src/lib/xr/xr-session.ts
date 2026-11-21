/**
 * What the XR page decides without the engine: where a URL lands in an experience, the configuration the engine
 * gets, and which of the page's states the engine's reports mean. The engine itself is lib/xr/xr-engine.ts.
 */

import type { XrGameConfiguration, XrPlugin } from '@scutoons/content';
import type { XrStatus } from '@scutoons/xr-engine/reader';

export type { XrStatus };

export interface XrPlace {
	scene: string;
	/** the target the URL names: the page says what to look for */
	target?: string;
}

type Row = Record<string, unknown>;
const row = (value: unknown): Row => (typeof value === 'object' && value !== null ? (value as Row) : {});
const text = (value: unknown): string | undefined => (typeof value === 'string' && value ? value : undefined);

/**
 * `/xr/<experience>` opens the first scene, `/xr/<experience>/<scene or target>` that scene; null = not in it.
 * The server checks the configuration only roughly, so nothing of its shape is taken for granted here.
 */
export function placeIn(configuration: XrGameConfiguration, at?: string): XrPlace | null {
	const scenes = configuration.scenes.flatMap((scene) => text(row(scene).id) ?? []);
	if (!at) {
		const initial = text(configuration.initialScene);
		const first = initial && scenes.includes(initial) ? initial : scenes[0];
		return first ? { scene: first } : null;
	}
	if (scenes.includes(at)) return { scene: at };
	for (const entry of configuration.entries.map(row)) {
		if (typeof entry.target !== 'object' || entry.target === null) continue;
		// a target's id defaults to its entry's
		if ((text(row(entry.target).id) ?? text(entry.id)) !== at) continue;
		const scene = text(entry.scene);
		return scene && scenes.includes(scene) ? { scene, target: at } : null;
	}
	return null;
}

/** the configuration with the document's plugins as modules the engine can import (`toUrl`: a blob URL of the code) */
export function withPlugins(configuration: XrGameConfiguration, plugins: readonly XrPlugin[], toUrl: (code: string) => string): XrGameConfiguration {
	return { ...configuration, plugins: plugins.map(({ id, api, code }) => ({ id, api, src: toUrl(code) })) };
}

export type XrView = 'loading' | 'ready' | 'starting' | 'scanning' | 'found' | 'paused' | 'failed';

export function viewOf(status: XrStatus, found: number): XrView {
	switch (status) {
		case 'idle':
		case 'loading':
			return 'loading';
		case 'running':
			return found > 0 ? 'found' : 'scanning';
		case 'error':
			return 'failed';
		default:
			return status;
	}
}

export type XrFailure = 'camera' | 'content' | 'other';

/** the engine's error code → what the page tells the reader */
export function failureOf(code?: string): XrFailure {
	if (code === 'ar-start-failed' || code === 'camera-not-responding') return 'camera';
	if (code === 'ar-load-failed') return 'content';
	return 'other';
}
