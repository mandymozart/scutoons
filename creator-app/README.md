# SCUTOONS Creator – the desktop app

A thin Electron window over the web creator (#860). It loads the deployed creator (`creatorUrl` in `config.json`,
`https://creator.scutoons.com`) – the same code and the same sign-in as the browser, updated by every deploy; nothing of
the creator is bundled. What the app adds: one instance, the `scutoons-creator://<workspace>/<address>` link
(server/schema/tables/deck-link.json), `scutoons-deck://` handed to the system, and the deck's download offered when no
deck is installed. It sits outside the npm workspaces, with its own `package-lock.json`.

## Run and build

    npm --prefix creator-app install
    npm --prefix creator-app run start     # from the checkout (TypeScript compiled with the repo's own tsc)
    npm --prefix creator-app run dist      # Windows only: dist/SCUTOONS Creator Setup <version>.exe, unsigned
    npm --prefix creator-app test

- `SCUTOONS_CREATOR_URL=http://127.0.0.1:<port>` points the app at a local creator (http only on this machine).
- `SCUTOONS_REGISTER_SCHEME=1` lets a start from the checkout register `scutoons-creator://` (off by default, so a test
  start never takes the links from an installed app).
- Downloads (Electron, NSIS) are cached in `creator-app/.cache/`; `dist/`, `out/` and `.cache/` are ignored.
- Sign-in: email and the one-time code typed in the app. The mail's magic link opens the browser, not the app.

## The window (#886)

As the deck's editor (`deck/editor/src/frame.rs`): no system title bar. `src/frame.ts` decides it, the preload hands the
page `frame` and the window's buttons (minimize, maximize / restore, close, the maximized state), nothing else; the main
process acts only on the creator's own main window.

- **Windows and Linux** (one path): a frameless window (`frame: false`; on Windows `thickFrame` keeps the resize edges,
  snapping and the shadow). The creator's bar is the title bar: its empty run is the drag region
  (`-webkit-app-region: drag`, a double click maximizes / restores), its controls are not, and the three window buttons
  (– □/❐ ×) sit at its right end after ☰. The deck's light Arch system has no system top bar either, so Linux matches
  Windows (owner 2026-11-24). Built and seen on Windows only; the Linux window is configured, untested.
- **macOS:** `titleBarStyle: 'hiddenInset'` – the system's traffic lights at the bar's left end (the bar leaves room
  for them), the bar the drag region, no own buttons. Configured, untested.
- A window the page opens (same origin) keeps the system's frame.

## The links

- Windows: the installer writes `HKCU\Software\Classes\scutoons-creator` (generated `out/installer.nsh`) and the
  uninstaller removes it; the app registers it again on every start.
- macOS: `CFBundleURLTypes` in Info.plist from `protocols` in `electron-builder.config.ts`; a link arrives as `open-url`.
- Arch Linux: the pacman package's `.desktop` file carries `MimeType=x-scheme-handler/scutoons-creator`; the app sets
  itself as the handler with `xdg-settings` on start.
- Whether a deck is installed: Windows reads `HKCR\scutoons-deck\shell\open\command`; macOS and Linux ask the system for
  the scheme's handler.

## For the owner

- **The deck's download:** `deckDownloadUrl` in `config.json` is empty – fill in the address (https). Until then the app
  says "No deck on this computer" without a download button.
- **Signing (Windows):** nothing is signed; SmartScreen warns on the first start. Add a code-signing certificate
  (`CSC_LINK` / `CSC_KEY_PASSWORD`, or `win.azureSignOptions`) and build again.
- **macOS:** built and tested only on a Mac: `npx electron-builder --mac --config electron-builder.config.ts` with a
  Developer ID certificate and notarization (`APPLE_ID`, `APPLE_APP_SPECIFIC_PASSWORD`, `APPLE_TEAM_ID`). Untested here.
- **Arch Linux:** `npx electron-builder --linux --config electron-builder.config.ts` on Linux (target `pacman`).
  Untested here.
- **Icon:** the creator's `creator/static/app-icon-512.png`; a dedicated `.ico` / `.icns` can replace it in the config.
