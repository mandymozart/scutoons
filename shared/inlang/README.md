# inlang plugins (vendored)

The two Paraglide/inlang plugins every app's `project.inlang/settings.json` loads, as files in the repo instead of from
cdn.jsdelivr.net (owner 2026-10-09: no dependency on third-party delivery – everything from our own deployment or
cdn.scutoons.com). Paths in `settings.json` are relative to the app folder: `../shared/inlang/<file>.js`.

- `plugin-message-format.js` – @inlang/plugin-message-format 4.x (the `messages/{locale}.json` files)
- `plugin-m-function-matcher.js` – @inlang/plugin-m-function-matcher 2.x (`m.key()` in the editor tooling)

Taken unchanged from the SDK's plugin cache (2026-10-09). To update: download the new `dist/index.js` of each package
once, replace the file, compile one app (`npx paraglide-js compile --project ./project.inlang --outdir <tmp>`).
