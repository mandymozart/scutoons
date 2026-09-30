# @scutoons/tokens

Design tokens for every SCUTOONS surface (viewer, creator, admin, the server's public pages): DESIGN.md Part 1 as CSS
custom properties, the self-hosted fonts and a small base stylesheet. No JavaScript, no build step: the apps' Vite
bundles the CSS and emits the font files.

## Public API

| Import | Contents |
|---|---|
| `@scutoons/tokens` / `@scutoons/tokens/index.css` | everything below, in this order |
| `@scutoons/tokens/tokens.css` | `:root` custom properties |
| `@scutoons/tokens/fonts.css` | `@font-face`: IBM Plex Mono 400/700 (+ italic 400/700), Bangers 400 (SIL OFL 1.1, `fonts/*-OFL.txt`) |
| `@scutoons/tokens/base.css` | `html` / `body` / headings / `p`, fluid root font size (14px < 480px, 16px, 24px ≥ 768px); form controls (`button`, `input`, `select`, `textarea`) `font: inherit` and `code`/`kbd`/`samp`/`pre` in `--font-mono`, both with `:where()` (specificity 0) |
| `@scutoons/tokens/fonts/*` | the font files (e.g. for a copy on the server) |

Tokens (see `tokens.css` for values and usage notes):

- Color: `--ink`, `--paper`, `--primary-cyan` (fills/glows only, never text), `--acid-purple|green|red` (VN VFX only),
  `--border-subtle` → `--border-muted` → `--border-hover` → `--ink`, `--surface-hover`, `--surface-panel`,
  `--surface-tint-cyan`, `--text-muted`. App UI states: `--state-error`, `--state-warning`.
  Legacy viewer aliases: `--primary-color`, `--color-acid-*`.
- Spacing: `--space-025` · `--space-05` · `--space-075` · `--space-1` · `--space-15` · `--space-3`.
- Radius: `--radius-none` (default, brand signature) · `--radius-sm` · `--radius-full`.
- Borders: `--border-width-hairline` (1px) · `--border-width-regular` (2px). Borders over shadows, no elevation.
- Type: `--font-mono`, `--font-lettering`, `--font-weight-regular|bold`, `--type-{body,button,label,caption,nav-micro}-{size,line}`,
  `--root-size-small|root-size|root-size-large`.
- Motion: `--duration-fast|base|slow` (0.1–0.3s), `--easing`.

## Example

```svelte
<!-- routes/+layout.svelte of an app -->
<script lang="ts">
	import '@scutoons/tokens';
</script>
```

```css
.panel {
	background: var(--paper);
	border: var(--border-width-regular) solid var(--ink);
	padding: var(--space-1);
}
.meta {
	color: var(--text-muted);
	font-size: var(--type-caption-size);
	line-height: var(--type-caption-line);
}
```

Vite apps must allow the shared folder for the dev server (`server.fs.allow: ['../shared']`), otherwise the font
files answer 403.

Every app root imports the whole package (`import '@scutoons/tokens'` in `routes/+layout.svelte`); no font CDN
anywhere, no hardcoded `'IBM Plex Mono'` stacks (use `var(--font-mono)` / `var(--font-lettering)`).

**qr** (plain PHP, no build step) serves a generated copy: `node qr/bin/sync-tokens.mjs` writes
`qr/public/assets/tokens/scutoons-tokens.css` (tokens + the Plex Mono `@font-face` rules + base) and the Plex Mono
woff2 files next to it (relative `url()`s). The copy is committed; run the script after changing this package.

## Check

`npm run check -w @scutoons/tokens`: every DESIGN.md token is defined, every `var()` in the package is defined, every
`url()` in `fonts.css` exists and every font file is used, and qr's copy is in sync (`qr/bin/sync-tokens.mjs --check`).

In a real browser: `npm run check:fonts` (= `node scripts/check-fonts.mjs --session`, dev servers from `npm run dev:up`)
loads the pages, dialogs and sheets of viewer, creator, admin, shop and qr in headless Chrome and asserts per page:
`document.fonts.check('1em "IBM Plex Mono"')` + a loaded Plex face, a Plex woff2 answered 200 on the network, no
font-CDN request, the computed `font-family` of every visible text and form control starts with IBM Plex Mono
(Bangers only as the lettering stack), and the font Chrome actually rendered (CDP `CSS.getPlatformFontsForNode`) for
texts with Latin/Cyrillic letters or digits. `--session` creates a throw-away admin + creator user in the local DB for
the signed-in pages and removes it; `--token=`/`SCUTOONS_TOKEN` uses an existing session; `--apps=qr,shop`,
`--only=<page>`, `--list`, `--verbose`. Symbols Plex doesn't have (☰ ⓘ ✕ →, emoji, CJK) fall back by design.
