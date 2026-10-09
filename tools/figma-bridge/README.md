# SCUTOONS Figma bridge

Full read / write access to the owner's open Figma file **without** the Figma MCP's monthly read cap (Starter: ~20
reads; owner 2026-10-09: "then we just build a SCUTOONS plugin bridge"). A private development plugin – not
published, no cost – talks to a relay on the owner's own computer; tools (the CLI, later the SCUTOONS MCP) talk to
the relay. Nothing leaves the computer. Concept: `agents/STORYBOARD.md` §9.

```
cli.mjs / SCUTOONS MCP ──HTTP──► relay.mjs (127.0.0.1:3056) ◄──WebSocket── plugin ui.html ──► code.js (Plugin API, the open file)
```

## Setup (once)

1. Figma **desktop** app → any design file → Plugins → Development → **Import plugin from manifest…** →
   `tools/figma-bridge/plugin/manifest.json` (from the checkout you work in).
2. `node tools/figma-bridge/relay.mjs` in a terminal (keep it open; Ctrl+C stops it).
3. In the file (Cat-Pope): Plugins → Development → **SCUTOONS Bridge**. The small window shows a green dot when it is
   connected. It acts as you, only while the window is open.

`node tools/figma-bridge/cli.mjs status` → `{"plugin":true}`.

## Commands

| CLI | Does |
| --- | --- |
| `ping` | file, page, user |
| `pages` | the file's pages (id, name) |
| `read --page "S1C3" [--depth 2]` / `read --node 555:2354` | the node tree: id, name, type, x / y / w / h, text, image hash, locked / hidden |
| `export --node 555:2354 --format png --scale 1 --to out.png` | a node as PNG / JPG / SVG (thumbnails for matching, slices for the CDN) |
| `draft storyboards/_out/s1c3/figma/scene-25.spec.json …` | builds / updates a scene's draft frame (from `scripts/storyboard/figma-draft.mjs`): locked `storyboard`, hidden `guides`, `text`, empty `layout` on top, the rail beside it. Re-runs replace shots by id and never touch `layout`. |

The draft code (`applyDraft` in `plugin/code.js`) is the same one `figma-draft.mjs` inlines for Figma's own MCP
(`use_figma`) – one implementation for both ways.

## In the SCUTOONS MCP

Creators (content write) get `figma_status`, `figma_read`, `figma_view` (a node as a PNG the AI can look at) and
`figma_place_draft` (`tools/mcp/src/figma.ts`) – the same relay; `SCUTOONS_FIGMA_PORT` if not 3056.

## Check

`node tools/figma-bridge/test.mjs` – relay + a stand-in plugin over a real WebSocket (handshake, > 64 KiB frames,
errors) and `applyDraft` against a small fake of the Plugin API (layers, order, lock, idempotence).
`node --experimental-transform-types --no-warnings tools/figma-bridge/test-mcp.mjs` – the MCP's figma_* tools through
the relay and a stand-in plugin. Neither needs Figma.

## Not yet

- **Not tried in Figma yet** (2026-10-09, built while the owner was away; Figma work waits for the owner). To verify on
  the first run: the manifest's `networkAccess` (`allowedDomains: ["none"]` + `devAllowedDomains` for the local
  WebSocket), `createNodeFromSvg` on the long schematics, fonts (Inter Regular / Bold).
- Export → asset manager (PLAN.md Track A): `export` + the MCP's `upload_asset`.
