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
