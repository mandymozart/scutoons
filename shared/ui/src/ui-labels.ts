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
}

const DEFAULTS: UiLabels = {
	reset: 'Reset',
	mixed: 'Mixed',
	pickColour: 'Pick a colour',
	palette: 'Palette',
	recent: 'Recent',
	shade: 'Saturation and brightness',
	hue: 'Hue',
	hex: 'Hex'
};

const KEY = Symbol.for('scutoons:ui-labels');

/** the app's words (getters are read when shown, so a language switch follows) */
export function provideUiLabels(labels: Partial<UiLabels>): void {
	setContext(KEY, labels);
}

/** call in a component's script: a reader of the provided words, English for any missing */
export function uiLabels(): (key: keyof UiLabels) => string {
	const provided = getContext<Partial<UiLabels> | undefined>(KEY);
	return (key) => provided?.[key] ?? DEFAULTS[key];
}
