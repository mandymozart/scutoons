// An XR experience (research/XR.md §4.2, §4.4): the catalog document `experience:<id>` the reader's `/xr/<id>` plays –
// OSCT's game configuration as OSCT wrote it, beside SCUTOONS's own fields. The configuration's contract (its guard,
// its types) is the engine's (@scutoons/xr-engine); this is only the document around it.
import type { Localized } from './items.ts';

/** What a new experience starts with in the editor (research/XR.md §4.2); the engine plays every type alike */
export const XR_EXPERIENCE_TYPES = ['book', 'record', 'poster', 'exhibition', 'concert-reading'] as const;
export type XrExperienceType = (typeof XR_EXPERIENCE_TYPES)[number];

/** The OSCT content version an experience was imported from */
export interface XrOsctSource {
	/** the OSCT experience's id */
	experience: string;
	/** the OSCT scene it was taken from, when only one */
	scene?: string;
	/** the published content version (build) */
	build: string;
	/** the content schema it was written in (`6.0`) */
	schema: string;
	/** the OSCT app version that wrote it (`6.3.2`) */
	app: string;
}

/**
 * OSCT's game configuration, unchanged: only the fields the server's shallow check reads are named here; the rest is
 * the engine's type (@scutoons/xr-engine), checked by its guard in the reader and the importer
 */
export interface XrGameConfiguration {
	version: {
		/** the content schema (`6.0`) */
		schema: string;
		[key: string]: unknown;
	};
	/** at least one */
	scenes: unknown[];
	entries: unknown[];
	[key: string]: unknown;
}

/** A compiled plugin of the configuration: an ES module as text (the reader imports it from a blob) */
export interface XrPlugin {
	/** `^[a-z][a-z0-9-]{0,39}$`, unique in the experience */
	id: string;
	/** the plugin API it was built for (`4.0`) */
	api: string;
	/** the module's source, at most 262144 bytes */
	code: string;
}

/** The catalog document `experience:<id>` (id `^[a-z][a-z0-9-]{1,63}$`) */
export interface XrExperience {
	id: string;
	type: XrExperienceType;
	/** { <locale>: text } */
	title: Localized;
	/** the toons it belongs to */
	toons: string[];
	source?: { osct: XrOsctSource };
	/**
	 * the CDN-root folder its files lie below (`xr/poster-catpope/e8f9fbfd81b2`, no slash at either end): the engine's base,
	 * so the configuration's `/assets/content/library/x.glb` is the asset `<assets>/assets/content/library/x.glb`
	 */
	assets: string;
	configuration: XrGameConfiguration;
	plugins: XrPlugin[];
}
