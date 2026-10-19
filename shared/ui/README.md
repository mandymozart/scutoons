# @scutoons/ui

The SCUTOONS Svelte UI kit for the creator, the admin and (step by step) the viewer: square, ink/paper, IBM Plex
Mono, 2px structural borders, cyan only as a fill. Follows DESIGN.md Part 1 and the "Admin UI patterns" in
agents/PLATFORM.md. Svelte 5 runes, snippets and callback props (no `<slot>`, no event dispatchers).

Styled only with `@scutoons/tokens` custom properties, so the app must `import '@scutoons/tokens'` once. No app
state, no routing, no paraglide: every visible string is a prop (defaults are English, e.g. `closeLabel = 'Close'`).

## Public API (`import … from '@scutoons/ui'`)

| Export | Props / usage |
|---|---|
| `Button` | `variant` `primary` \| `secondary` \| `clear`, `size` `md` \| `sm` (compact for bars: 1rem, padding `space-05 space-15`; the viewer's concept, secondary outline `border-muted` → ink on hover), `href` (renders `<a>`), `disabled`, any button/anchor attribute; children = label |
| `IconButton` | `label` (required: aria-label + title), `icon` (image URL, 16px) or children, `pressed`; 2rem circle, cyan fill on hover/pressed |
| `CloseIcon` | `size` (the adjacent headline's font-size, default `--type-label-size`), `stroke` 2 (default) / 1; THE close cross of every app: square, as high as the headline's capitals (0.698 × size), 2px (1px on thin-line UI) – DESIGN.md "Close cross" |
| `CloseButton` | `label` (required), `size`, `stroke`; a frameless `--control-height` square with the `CloseIcon` centred (Sheet, InfoPanel, SplitView, toasts) |
| `Field` | `label`, `hint`, `error`, `required`, `id`; `children({ id, describedby, invalid })` renders the app's input/select/textarea (styled inside `.field`) |
| `Panel` | framed box: `title`, `hint`, `actions` snippet, `quiet` (surface-panel, no border) |
| `Section` | entity-detail section: `title`, `hint`, `media` snippet (leading visual, e.g. a `ToonAvatar`), `actions` snippet |
| `Sheet` | modal on native `<dialog>`: `bind:open`, `title`, `side` `center` \| `bottom` \| `right`, `closeLabel`, `onclose`, `footer` snippet (sticky action bar) |
| `Tabs` | `tabs: { id, label, disabled? }[]`, `bind:value`, `label`, `onchange`, `children(activeId)` = active panel; arrow keys / Home / End |
| `Toggle` | switch: `bind:checked`, `label`, `hint`, `disabled`, `onchange(checked)` |
| `Badge` | `tone` `neutral` \| `ink` \| `accent` \| `warning` \| `error`; children = text |
| `Menu` | dropdown: `label`, `trigger` snippet, `align` `start` \| `end`, `drop` `down` \| `up` (opens above the trigger – a menu in the bottom ActionBar), `bind:open`; children = the items (buttons / links, left-aligned); the popup as wide as its widest item (12rem … 40rem), kept inside the window; closes on outside click, Escape, item click |
| `createToaster({ timeout?, max? })` | runes store: `items`, `addToast({ message, type?, timeout? })`, `dismiss(id)`, `clear()`; errors stay until dismissed |
| `ToastStack` | `toaster`, `dismissLabel`; fixed bottom-left, above the ActionBar and above the AppShell's bottom region while it shows (`--sc-shell-bottom-lift`) |
| `clickOutside(handler, { escape? })` | attachment: `<div {@attach clickOutside(close)}>` |
| `ActionBar` | fixed bottom action bar (PLATFORM.md "Bottom action bar", mandatory for admin + creator): mirrors the top bar (height `--action-bar-height`, default `--tool-bar-height`; paper; 2px ink rule on top; full width; always visible). Snippets `start` (pagination, filters, search, status), `secondary` (secondary actions), `end` (primary action, right-most); below `breakpoint` (1024px) `start` + `secondary` move into a "More" panel opening upwards, `end` stays. Adds the bottom padding to `body` while it is on the page. Props `label`, `moreLabel`. Use compact controls inside (`SearchField compact`, `Pager compact`). **Info:** pass the page's help as the `info` snippet → an "ⓘ Info" toggle (left-most, also on narrow screens) opens an `InfoPanel` with it; `infoTitle`, `infoHref` (link to the full style guide), `infoLabels` `{ toggle, close, guide }`, `bind:infoOpen`, `infoKey` (localStorage key; closed by default, the user's choice is remembered across pages) |
| `InfoPanel` | the one place for a page's help (owner rule: explanations, conventions, glossaries, style guide go here, only short labels and errors stay inline): a page panel of the AppShell's right dock (`kind: 'info'`, outermost; drags wider, folds to a rail, maximizes – agents/DOCKING.md). Props `title`, `closeLabel`, `href` + `hrefLabel` (full guide link), `onclose` (× and Escape), `id`, `foldLabel` / `maximizeLabel` / `restoreLabel`; children = plain HTML (`h3`, `p`, `ul`, `dl`, `code`, `kbd` are styled). Normally rendered by `ActionBar`'s `info` |
| `AppShell` | page frame of the tools (admin, creator, embassy, the storyboard studio – agents/DOCKING.md): snippets `topbar`, `sidebar` (left, `sidebarTag` nav/aside/div, `sidebarLabel`, `sidebarId`, `sidebarWidth`), children = content; the right dock shows the panels opened with `useDock` (`dockLabel`, `dockResizeLabel`, `dockUnfoldLabel`, `minContent`; the main's rail `pageLabel` / `pageFoldLabel` / `pageUnfoldLabel`). The region borders always reach the fixed `ActionBar`: `mode="page"` (document scrolls; sticky top bar; sidebar / dock sticky, exactly viewport − top bar − ActionBar tall, scroll on their own) or `mode="fill"` (viewport − ActionBar, no document scroll, every region scrolls itself – editors). Reserves `--action-bar-height` itself instead of the ActionBar's `body` padding; measures the top bar (`--sc-shell-top`). Sidebar: `resizeKey` (+ `resizeLabel`) drags it, `foldable` (+ `railLabel`, `foldLabel`, `unfoldLabel`, `foldWhen`) folds it to a rail; `collapsible` + `sidebarOpen` + `onclose` (+ `closeLabel`): off-canvas drawer with scrim below 768px. **Bottom region** (fill mode only, from 768px): `bottom` snippet + `bottomSize` (a `panelHeight`), `bottomLabel`, `bottomIcon`, `bottomResizeLabel`, `bottomUnfoldLabel` – a row under the content and the dock, the sidebar keeps its full height; its top edge a `Divider`, folds to a horizontal `Rail` (the page's `FoldButton side="bottom"` calls `bottomSize.fold`), a rail while a region is maximized, stays mounted (agents/DOCKING.md "Bottom region") |
| `useDock(() => DockPanel \| null)` · `shellDock` | docks a page panel in the AppShell's right dock while the component is mounted: `{ kind, panel \| content, size: panelWidth(…, { foldable: true }), title, icon, order, min, close }`; one per kind in fixed places (`DOCK_ORDER`; an app's own kinds by `order` 0–499, before Info); `min` (rem, default 12) = the floor it squeezes to before the least recently used folds (DOCKING "Running out of room"); `shellDock.maximize` / `restore` / `foldMain` (agents/DOCKING.md) |
| `panelWidth(key, normalRem = 24, rightEdge?, { foldable? })` · `panelHeight(key, normalRem = 10)` | a docked region's remembered size for its `Divider`: a right panel's width (normal … half the window), the AppShell bottom region's height (normal … half the shell's frame – `room` / `edge` set by the shell; `fold` / `folded`, the fold is the page's to remember); `resizable` / `dockRegion` below them; `Divider`, `FoldButton`, `Rail`, `DockFold`, `MaximizeButton` are the parts |
| `Timeline` | the one time axis (`timeline.svelte` + the pure model `timeline.ts`; research/TIMELINE.md): lanes of items in ms on a linear axis with zoom, a ruler + markers + the playhead; callbacks `onseek` / `onselect` / `ontrim` – read-only without them (a lane with `selectable: false`: a press on an item seeks); `reveal` (+ `revealFill`) brings a range into view – also when the app sets `bind:zoom`; `follow`; hover (mouse / pen, never a finger): `preview(item, lane, t)` snippet = a box next to the pointer over an item (the studio: the clip's first frame; on the item's top / bottom edge, never over it, sliding along the timeline, in the top layer, inside the window; the items then have no title tooltip), `onhover(t \| null)` = the time under the pointer on the ruler / lanes (an app's hover line) – both end on leave, press / drag / trim, scroll, zoom, Escape, new lanes and when it unmounts; one `role="group"` (the bottom region is the landmark); no transport inside (the app owns playback and passes the playhead). Lives in the AppShell's bottom region (the storyboard studio now, the creator later) |
| `DataTable` | generic (`T`): `columns: Column[]` (`{ id, label, sortable?, align?, width?, secondary? }` – secondary hides < 1600px, the tools' 24px type needs the room), `rows`, `rowKey`, `cell(row, column)` snippet, `sort` (`field`\|`-field`) + `sortHref(sort)` (URL-driven) or `onsort`, `defaultSortDir`, `selectable` + `bind:selected` + `bulk(keys)` snippet (bar above the table), `onrowclick` (click / Enter), `activeKey`, `rowClass`, `loading` (no rows = an empty body, no text: DESIGN.md "Empty states"), `caption`, `labels`; `nextSort(current, field)` |
| `Pager` | URL-driven: `page`, `pageSize`, `total`, `href(page)`, optional `sizeHref(size)` + `pageSizes`, `compact` (one line, for the ActionBar), `labels` |
| `SearchField` | debounced search input: `label`, `value` (follows the URL), `onsearch(q)`, `delay`, `compact` (label as placeholder / accessible name) |
| `entityLabel(key, labels?)` · `ENTITY_LABELS` | the ONE mapping from a content entity's storage key to its display name (no file names in the UI): `chapter2.json` → "Chapter 2", `season1.json` → "Season 1", `chapter2/scene-14.json` → "Scene 14", `toon.json` → "Toon settings", `characters.json` / `moods.json` / `asset-requests.json` → "Characters" / "Moods" / "Wanted images", cast, credits, people, roles, the toon's index → "Languages"; toon / locale prefixes are ignored. `labels` = the app's translations (`chapter(n)`, `season(n)`, `scene(label)` + strings); the data refactor switches keys here only |
| `readList(url, { pageSize, sort, filters })` · `listHref(url, patch)` · `listParams(state)` | pure URL helpers for `?page=&page_size=&sort=&q=&<filter>=` lists (a change resets `page`) |
| `SplitView` | list + detail: `open`, `title`, `onclose`, `list` / `detail` / `actions` snippets, `closeLabel`, `foldLabel` / `maximizeLabel` / `restoreLabel`; the list is the page, the detail a panel of the AppShell's right dock (`kind: 'detail'`, `sc-panel-width:detail`), on phones it covers the page |
| `CommandPalette` | `bind:open`, `commands: Command[]` (`{ id, label, hint?, group?, keywords?, run }`), `dynamic(query)` (query-dependent commands first), `hotkeys` (Ctrl/⌘+K, `/` outside text fields), labels; `matchCommand` |
| `PaletteTrigger` | the top bar's "search or jump to" button that opens the palette: `label`, `shortcut` (kbd hint, default `Ctrl K`), `compact` (icon + kbd, no label text – for crowded bars); below 768px only the icon, `onclick`; search icon + label + kbd in a 2px ink frame (`data-testid="palette-trigger"`) |
| `linkCommands(links, go, group?)` · `languageCommand({ locales, current, setLocale, label, group })` · `signOutCommand({ label, run, group })` · `otherLocale` | the palette commands every tool has: "go to" from nav links (`{ id, label, href, hint?, keywords?, group? }`), language switch, sign out |
| `Skeleton` | loading placeholder: `variant` `text` (`lines`) \| `block` (`height`) \| `table` (`rows`, `columns`), `label`; static bars, no shimmer |
| `Chart` | plain SVG `kind` `line` \| `bar`, `labels` (x), `series: { label, values }[]` (≤ 4), `height`, `ariaLabel`, `formatLabel`, `formatValue`; one y-axis from 0, cyan only as fill, series told apart by fill/outline/hatch or dash pattern + legend; hover / arrow keys = crosshair + values |
| `ToonAvatar` | a toon's navigation avatar (CDN `<toonId>/avatar.webp`, 512² transparent WebP): `src`, `alt` (default `''`), `name` (placeholder letter, default `alt`), `size` (CSS length for width + height, default `2.5rem`); `radius-sm`, no border, `object-fit: contain`, `loading="lazy"`; no `src` or a load error → first letter on `surface-hover` |

## Example

```svelte
<script lang="ts">
	import '@scutoons/tokens';
	import { Button, Field, Sheet, Badge, ToastStack, createToaster } from '@scutoons/ui';

	const toaster = createToaster();
	let open = $state(false);
	let email = $state('');
</script>

<Button onclick={() => (open = true)}>Revoke code</Button>
<Badge tone="error">revoked</Badge>

<Sheet bind:open title="Revoke code" side="right">
	<Field label="Reason" hint="Shown in the event log">
		{#snippet children({ id, describedby })}
			<input {id} aria-describedby={describedby} bind:value={email} />
		{/snippet}
	</Field>
	{#snippet footer()}
		<Button variant="secondary" onclick={() => (open = false)}>Cancel</Button>
		<Button onclick={() => toaster.addToast({ message: 'Code revoked', type: 'success' })}>Revoke</Button>
	{/snippet}
</Sheet>

<ToastStack {toaster} />
```

Page help in the Info panel (every admin + creator page):

```svelte
<ActionBar infoTitle="Codes" infoHref="https://creator.scutoons.com/guide" infoLabels={{ toggle: 'Info', close: 'Close', guide: 'Style guide' }}>
	{#snippet info()}
		<h3>Filters</h3>
		<p>Search matches the code, the email and the edition.</p>
	{/snippet}
	{#snippet end()}<Button>Generate</Button>{/snippet}
</ActionBar>
```

## Check

`npm run check -w @scutoons/ui` = `svelte-check --fail-on-warnings`.

Admin example (URL-driven list):

```svelte
<script lang="ts">
	import { page } from '$app/state';
	import { DataTable, Pager, readList, listHref, listParams } from '@scutoons/ui';
	const query = $derived(readList(page.url, { sort: '-created_at', filters: ['status'] }));
	// load: api.list(listParams(query)) → { items, pagination }
</script>

<DataTable {columns} rows={items} rowKey={(r) => r.code} sort={query.sort} sortHref={(sort) => listHref(page.url, { sort })} />
<Pager {...pagination} href={(p) => listHref(page.url, { page: p })} />
```
