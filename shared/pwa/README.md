# @scutoons/pwa

What makes a SCUTOONS app an installable web app – concept and decisions: `agents/research/PWA.md` (the reader) and
`agents/research/PWA-APPS.md` (creator, admin, embassy).

- `install-rules.ts` (pure, `npm test -w @scutoons/pwa`) + `install.svelte.ts`: when to offer "Install … as an app",
  what this browser can do (`installWay`). Every app makes one `new Install({ key })`.
- Tools only: `tool-worker.ts` (the app's `src/service-worker.ts` calls `startToolWorker(self, …)` – app shell, no content
  cache), `tool-pwa.svelte.ts` (registration, update, kill switch `VITE_SW=off`), `install-bar.svelte`, `update-bar.svelte`,
  `labels.ts` (de/en).
