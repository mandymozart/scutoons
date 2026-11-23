/**
 * What an app sees of the XR engine: the types of `./reader`, with no import of OSCT's files. The package serves this
 * file as the entry's types, so an app's own TypeScript settings never read the engine's sources (they are checked
 * here, with the engine's settings). src/reader.ts is the code and must fit these declarations.
 */

/** what the scene is doing (the engine's `ArStatus`) */
export type XrStatus = 'idle' | 'loading' | 'ready' | 'starting' | 'running' | 'paused' | 'error';

export interface XrHandlers {
	/** `code`: the engine's error code with the status `error` */
	status(status: XrStatus, code?: string): void;
	/** how many targets the camera sees right now */
	found(count: number): void;
	/** the line of the video that plays, null when there is none */
	subtitle(text: string | null): void;
}

export interface XrOptions {
	/** where the app serves the image tracking's files (xr.js, xr-tracking.js), with a trailing slash */
	xr8Base: string;
	/** where the configuration's `/assets/content/…` paths lie: the folder's address, without a trailing slash */
	assetsBase: string;
	/** the language subtitles show in, asked while a video plays */
	language: () => string;
	handlers: XrHandlers;
}

export interface XrRun {
	/** camera and tracking on: called from the reader's tap, the browser asks for the camera then */
	start(): Promise<void>;
	/** camera released */
	stop(): Promise<void>;
	dispose(): Promise<void>;
}

/** a compiled plugin of the experience: an ES module as text */
export interface XrPluginCode {
	id: string;
	api: string;
	code: string;
}

/** the scene, or why the configuration is not one this engine can read (one line each) */
export type XrOpened = { run: XrRun } | { problems: string[] };

/**
 * Reads `configuration` (OSCT's game configuration, without plugins) with `plugins` as its plugins and builds `scene`
 * in `container`. What the reader found lives as long as the page.
 */
export declare function openXr(container: HTMLElement, configuration: unknown, plugins: readonly XrPluginCode[], scene: string, options: XrOptions): Promise<XrOpened>;
