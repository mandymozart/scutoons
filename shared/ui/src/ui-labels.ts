// The words the shared field controls show (reset, colour popover, "Mixed"): shared/ui has no paraglide, so the app
// provides them once near its root (`provideUiLabels` in a layout); every ScrubNumber / ColorField / PropSelect below
// reads them from the context – also the ones another feature built without passing labels. English without a provider.
import { getContext, setContext } from 'svelte';

export interface UiLabels {
	/** the reset button behind a field: back to the default (the placeholder) */
	reset: string;
	/** a number field that stands for several different values */
	mixed: string;
	/** the colour popover: its button, sections and controls */
	pickColour: string;
	palette: string;
	recent: string;
	shade: string;
	hue: string;
	hex: string;
	/** the eyedropper: any colour on the screen (the browser's EyeDropper) / a point of the app's picture (fallback) */
	eyedropper: string;
	eyedropperPicture: string;
}

const DEFAULTS: UiLabels = {
	reset: 'Reset',
	mixed: 'Mixed',
	pickColour: 'Pick a colour',
	palette: 'Palette',
	recent: 'Recent',
	shade: 'Saturation and brightness',
	hue: 'Hue',
	hex: 'Hex',
	eyedropper: 'Pick a colour from the screen',
	eyedropperPicture: 'Pick a colour from the picture'
};

const KEY = Symbol.for('scutoons:ui-labels');

/** the app's words (getters are read when shown, so a language switch follows) */
export function provideUiLabels(labels: Partial<UiLabels>): void {
	setContext(KEY, labels);
}

/**
 * The eyedropper's fallback where the browser has no EyeDropper (Firefox, Safari, phones): the app provides a getter
 * that returns a sampler while it can offer one (the creator: while a slice canvas is open) – the sampler resolves to a
 * picked `#rrggbb`, or null when cancelled. ColorField offers the eyedropper when either exists.
 */
const SAMPLER = Symbol.for('scutoons:colour-sampler');
export type ColourSampler = () => Promise<string | null>;

export function provideColourSampler(current: () => ColourSampler | null): void {
	setContext(SAMPLER, current);
}

export function colourSampler(): () => ColourSampler | null {
	return getContext<(() => ColourSampler | null) | undefined>(SAMPLER) ?? (() => null);
}

/** call in a component's script: a reader of the provided words, English for any missing */
export function uiLabels(): (key: keyof UiLabels) => string {
	const provided = getContext<Partial<UiLabels> | undefined>(KEY);
	return (key) => provided?.[key] ?? DEFAULTS[key];
}
