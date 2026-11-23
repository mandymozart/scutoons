// OSCT's editor compiles plugin sources with sucrase; its stage files reach that file only through types
// (sky.ts → library-place.ts → api.ts → publish-plugins.ts → plugins/compile.ts). Nothing here compiles a plugin,
// so the package is declared, not installed.
declare module 'sucrase' {
	export function transform(code: string, options: Record<string, unknown>): { code: string };
}
