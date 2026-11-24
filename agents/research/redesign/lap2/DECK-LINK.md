<!-- status: quick research for lap 2 (owner 2026-11-24), not decided – how a workspace card in one app opens the other app on that workspace and its last content. -->
# Creator ↔ deck: one switcher, opening each other

Owner 2026-11-24: *"workspaces from the deck or the creator should be shown vice versa to create a more seamless experience. So those workspace links will open the respective deck if active, jump into the workspace and the last content it had."* Asked: Windows, iOS (read: macOS – there is no native deck on iOS) and Linux; the deck has a Chromium.

## Two needs, two answers
**1. Knowing what the other app has open – the server, no local tricks.** Both apps are signed in to the same account. #838 already keeps one state per member and device (`member_sessions`, `GET/PUT /api/me/session`): the deck becomes one more device there – it writes its workspace, its open item and when it was last alive. Then each switcher shows the other app's workspaces with their last content (and #842's stills for the pictures), on any OS, and knows whether the deck was active a moment ago.

**2. Bringing the other app up on that workspace.**
- **Creator → deck, the deck running:** the jump goes as a message through the account (the server, or the relay the deck already joins): "open workspace Scene with scene:avatar-home". No OS integration; works the same on Windows, macOS and Linux.
- **Creator → deck, the deck not running:** a URL scheme the deck's installer registers: `scutoons-deck://<workspace>[/<address>]`, e.g. `scutoons-deck://stage/scene:avatar-home` (built, #859: the grammar, the deck's cards and its device row are `server/schema/tables/deck-link.json` with `deck-link.vectors.json`; the workspace an item opens in is the core's `library::open`). The browser asks once ("Open SCUTOONS Deck?").
  - Windows: `HKCU\Software\Classes\scutoons-deck` with `URL Protocol` and `shell\open\command` = `deck-editor.exe "%1"`.
  - macOS: `CFBundleURLTypes` in the app's Info.plist; a running app gets the URL as an event.
  - Linux – **Arch Linux** (owner 2026-11-24; the deck OS and a desktop): the same freedesktop way, the deck's package ships the file; a `.desktop` file with `MimeType=x-scheme-handler/scutoons-deck`, made default with `xdg-mime`.
  - On Windows and Linux the OS starts a second process with the URL: it hands it to the running deck (a single-instance lock + a local pipe/socket) and quits – so the scheme also works when the deck runs.
- **Deck → creator:** the deck opens the creator's address for that workspace and item (`https://creator.scutoons.com/scenes/avatar-home`) – in the default browser on Windows/macOS, in its own Chromium `--app` window on the deck OS (it already does this from OPEN CREATOR). An open creator tab takes the jump itself (a new tab hands it to the existing one and closes – BroadcastChannel), and #838 restores the rest.
- **Not chosen:** the creator page calling the deck on `http://127.0.0.1:<port>`. Chrome 142+ asks the user for "local network access" on every site that does it, Safari and Firefox differ; on the deck OS a managed policy could allow it, elsewhere it is a prompt. The server message + the URL scheme need no such permission.

## With the creator as an Electron app (decided 2026-11-24)
Both apps are then programs on the machine: the creator registers `scutoons-creator://` the same way as the deck's scheme (Electron's own `setAsDefaultProtocolClient` + its single-instance lock), and the two can talk directly on the machine (a local socket / pipe) without any browser permission. The server message stays for a deck on another machine. Built (#860): `creator-app/` loads the deployed creator; `scutoons-creator://<workspace>/<address>` is the same grammar under the creator's scheme (deck-link.json `creator`, its vectors `creator_links` and `read`), the workspace the creator's own name for it.

## What it means for the screens
- One switcher list in both apps: the creator's workspaces and the deck's (Scene · Rigging · Show), each card with its still and its last content; a deck card says whether the deck is running on this machine (from its last report) – opening it jumps there or starts it.
- No Views on the cards (Views live only in the ☰ – owner 2026-11-24).

Sources: Chrome, Local Network Access (developer.chrome.com/blog/local-network-access, launched in Chrome 142); Electron, Deep Links (electronjs.org/docs/latest/tutorial/launch-app-from-url-in-another-app); Microsoft, Registering an Application to a URI Scheme; Wails, Custom URL Protocols (v3.wails.io/guides/distribution/custom-protocols); deck/os/README.md (the deck OS's Chromium `--app` window).
