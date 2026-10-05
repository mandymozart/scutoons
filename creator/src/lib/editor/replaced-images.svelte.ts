// Images replaced in place in this session (the slice import / a drop onto a slice, owner 2026-10-06): the file keeps its
// URL but has new bytes, which the browser would keep showing from its cache. Their URLs get `?v=<time>` so the slice
// list and the canvas load them again (like the characters import's `bust`).

const replaced = $state<Record<string, number>>({});

export const replacedImages = {
	/** a CDN-root path (`catpope/s1/c1/s1c1-slice-26.webp`) got new bytes */
	mark(path: string) {
		replaced[path] = Date.now();
	},
	/** the URL of a CDN-root path, made fresh when the file was replaced */
	src(url: string, path: string) {
		const v = replaced[path];
		return v ? `${url}${url.includes('?') ? '&' : '?'}v=${v}` : url;
	}
};
