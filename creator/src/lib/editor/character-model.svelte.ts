// A character look's model as the 3D place and the Rig tab read it: the file's bytes once, the core's facts of them
// (bones, clips, spots, what it lacks – core-model.ts), and the clip the place plays. The facts are the core's; this
// holds and loads.
import { asset } from '$lib/utils/cdn.ts';
import { m } from '$lib/paraglide/messages.js';
import { modelFacts, type ModelFacts } from './core-model.ts';

export class CharacterModel {
	/** the model's CDN-root path ('' = none) */
	path = $state('');
	bytes = $state.raw<ArrayBuffer | null>(null);
	facts = $state.raw<ModelFacts | null>(null);
	/** why it could not be read, in words (the file, or the core's answer) */
	why = $state('');
	loading = $state(false);
	/** the clip playing (an index into `facts.clips`), null = the rest pose */
	clip = $state<number | null>(null);
	playing = $state(false);
	/** seconds into the clip */
	time = $state(0);
	skeleton = $state(true);
	#asked = '';

	/** reads `path` (again only when it changed) */
	async read(path: string) {
		if (path === this.#asked) return;
		this.#asked = path;
		this.path = path;
		this.bytes = null;
		this.facts = null;
		this.why = '';
		this.clip = null;
		this.playing = false;
		this.time = 0;
		if (!path) return;
		this.loading = true;
		try {
			const response = await fetch(/^https?:/.test(path) ? path : asset(path));
			if (!response.ok) {
				if (path === this.#asked) this.why = response.status === 404 ? m['character.rig_not_found']() : m['character.rig_load_failed']();
				return;
			}
			const bytes = await response.arrayBuffer();
			if (path !== this.#asked) return;
			const answer = await modelFacts(new Uint8Array(bytes));
			if (path !== this.#asked) return;
			if (answer.ok) {
				this.facts = answer;
				this.bytes = bytes;
			} else this.why = answer.why;
		} catch {
			if (path === this.#asked) this.why = m['character.rig_load_failed']();
		} finally {
			if (path === this.#asked) this.loading = false;
		}
	}

	/** a clip plays from its start; the playing one pauses or plays on */
	play(clip: number) {
		if (this.clip === clip) this.playing = !this.playing;
		else {
			this.clip = clip;
			this.time = 0;
			this.playing = true;
		}
	}

	/** back to the rest pose */
	rest() {
		this.clip = null;
		this.playing = false;
		this.time = 0;
	}
}
