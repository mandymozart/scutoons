// The eyedropper's fallback on the slice canvas (owner 2026-10-18; picture-pick.ts reads the pixels): a ColorField asks
// (`start`, through @scutoons/ui `provideColourSampler` in the root layout), the open slice canvas turns into a picking
// surface, a tap answers with the colour (`finish(hex)`), Esc with null. One per page.
import { shared } from '$lib/utils/shared.ts';

class PicturePick {
	/** the canvas waits for a tap */
	active = $state(false);
	/** slice canvases on the page (the fallback is offered only while one is open) */
	canvases = $state(0);
	#resolve: ((hex: string | null) => void) | null = null;

	start(): Promise<string | null> {
		this.finish(null);
		this.active = true;
		return new Promise((resolve) => (this.#resolve = resolve));
	}

	finish(hex: string | null) {
		const resolve = this.#resolve;
		this.#resolve = null;
		this.active = false;
		resolve?.(hex);
	}
}

export const picturePick = shared('picture-pick', () => new PicturePick());
