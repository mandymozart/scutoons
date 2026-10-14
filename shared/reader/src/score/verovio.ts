/// <reference path="./verovio-modules.d.ts" />
/**
 * Verovio (LGPL, its own lazy chunk – loaded only when a score opens; research/SCORES.md §4): MusicXML → SVG pages.
 * One toolkit for the whole app, created on first use.
 */
export interface ScoreToolkit {
	setOptions(options: Record<string, unknown>): void;
	loadData(data: string): boolean;
	redoLayout(options?: Record<string, unknown>): void;
	getPageCount(): number;
	getPageWithElement(id: string): number;
	renderToSVG(page: number): string;
	/** when each note starts / stops (ms) – the player's clock (score/synth.ts) */
	renderToTimemap(options?: Record<string, unknown>): { tstamp: number; on?: string[]; off?: string[] }[];
	getMIDIValuesForElement(id: string): { pitch?: number; time?: number; duration?: number };
}

let toolkit: Promise<ScoreToolkit> | null = null;

export function loadToolkit(): Promise<ScoreToolkit> {
	toolkit ??= Promise.all([import('verovio/wasm'), import('verovio/esm')])
		.then(async ([{ default: createModule }, { VerovioToolkit }]) => new VerovioToolkit(await createModule()) as ScoreToolkit)
		.catch((err) => {
			toolkit = null;
			throw err;
		});
	return toolkit;
}

/**
 * Repairs what MuseScore exports and Verovio can't take (it aborts inside its engine – a WebAssembly exception):
 * a chord symbol's added degree with an empty value (`<degree><degree-value></degree-value>…</degree>`, "No He Said"
 * bar 12). Only that empty addition goes; the chord itself stays. The stored file is never changed.
 */
export function cleanMusicXml(xml: string): string {
	return xml.replace(/<degree>\s*<degree-value>\s*<\/degree-value>[\s\S]*?<\/degree>/g, '');
}
