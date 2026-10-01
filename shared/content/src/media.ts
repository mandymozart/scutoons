// Language-independent media (owner rule 2026-10-01, agents/PLAN.md RULES, agents/DATA.md "Language-independent media"):
// images, videos and audio files are shared by every language; text lives only in text fields. A media path with a
// locale segment (`images/1x/de/…`, `…_en.webp`, `…-de.mp3`) is a smell: the file probably carries language (baked
// lettering) and has a twin per language. The validation warns on it in every entity (`mediaLocaleProblems`).
import type { Path } from './paths.ts';

/** File extensions that count as media (images, video, audio) */
export const MEDIA_EXTENSION = /\.(jpe?g|png|webp|gif|svg|avif|mp4|webm|mov|m4v|mp3|m4a|aac|ogg|oga|wav|flac)$/i;

/** Language codes a locale segment may carry (the content languages plus the common ISO 639-1 ones and the old jp / kr) */
const LOCALE_CODES = ['de', 'en', 'es', 'fr', 'it', 'nl', 'pl', 'pt', 'ru', 'tr', 'ja', 'jp', 'ko', 'kr', 'zh'];
const CODE = `(?:${LOCALE_CODES.join('|')})(?:[-_][A-Za-z]{2})?`;
/** a folder named like a locale (`de/`, `en-US/`) or a file name ending in one (`_de.`, `-en.`, `.de.`) */
const LOCALE_SEGMENT = new RegExp(`(?:^|/)(${CODE})/|[._-](${CODE})\\.[a-z0-9]+$`, 'i');

/** The locale segment of a media path (`de`, `en` …), or null when the path has none / is not a media path */
export function mediaLocaleSegment(path: string): string | null {
	if (typeof path !== 'string' || !MEDIA_EXTENSION.test(path.trim())) return null;
	const match = LOCALE_SEGMENT.exec(path.trim());
	return match ? (match[1] ?? match[2]).toLowerCase() : null;
}

/** Every media path with a locale segment anywhere in a document (`[path, value, locale]`) */
export function mediaPathsWithLocale(data: unknown, path: Path = []): [Path, string, string][] {
	const out: [Path, string, string][] = [];
	const walk = (value: unknown, at: Path) => {
		if (typeof value === 'string') {
			const locale = mediaLocaleSegment(value);
			if (locale) out.push([at, value, locale]);
		} else if (Array.isArray(value)) value.forEach((v, i) => walk(v, [...at, i]));
		else if (value && typeof value === 'object') for (const [k, v] of Object.entries(value)) walk(v, [...at, k]);
	};
	walk(data, path);
	return out;
}

/** The warning text for one such path */
export const mediaLocaleMessage = (value: string, locale: string) =>
	`media path "${value}" has a locale segment (${locale}): media are shared by every language – lettering belongs in a text field, not in the file`;
