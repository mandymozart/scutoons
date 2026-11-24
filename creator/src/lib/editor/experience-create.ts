// A new poster experience through the one save path every document takes: a free id from the title, the first draft of
// `experience:<id>` (the server checks it like any experience); the page opens it after.
import { NEUTRAL_LOCALE } from '@scutoons/content';
import { XR_CONTENT_SCHEMA } from '@scutoons/xr-engine/tree';
import { neutralDoc } from '$lib/editor/languages.ts';
import { experienceId, experienceKey, newPoster } from '$lib/editor/experiences.ts';
import { m } from '$lib/paraglide/messages.js';

/** saves a new poster titled `title` (in `locale`) and answers its id */
export async function createPoster(title: string, locale: string): Promise<string> {
	const taken = new Set<string>();
	for (;;) {
		const id = experienceId(title, taken);
		const doc = neutralDoc(experienceKey(id));
		await doc.load();
		if (doc.error) throw new Error(doc.error);
		if (doc.source[NEUTRAL_LOCALE] !== 'missing') {
			taken.add(id);
			continue;
		}
		const label = m['creator.experiences_new_surface']();
		doc.create(() => newPoster({ id, title, locale, toon: null, label, schema: XR_CONTENT_SCHEMA, now: new Date().toISOString() }));
		await doc.save(title);
		return id;
	}
}
