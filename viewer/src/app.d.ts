// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		/** shallow routing's history state: #478 the browser deck's stage only (live-deck/deck-stage-only.svelte) */
		interface PageState {
			deckStageOnly?: boolean;
		}
		// interface Platform {}
	}
	/** VITE_SW=off at build time: the service worker's kill switch (vite.config.ts, agents/research/PWA.md) */
	const __SW_OFF__: boolean;
}

export {};
