// verovio ships no types (6.3.0): only what score/verovio.ts uses
declare module 'verovio/wasm' {
	const createVerovioModule: () => Promise<unknown>;
	export default createVerovioModule;
}

declare module 'verovio/esm' {
	export class VerovioToolkit {
		constructor(module: unknown);
	}
}
