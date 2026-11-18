// The docs are public and plain pages: rendered at build time (no sign-in, no app frame), unlike the creator's SPA.
export const ssr = true;
export const prerender = true;
// /docs/ and /docs/<slug>/ are folders with an index.html: the host serves them as they are, past the SPA's fallback.
export const trailingSlash = 'always';
