# @scutoons/ui

The SCUTOONS Svelte UI kit for the creator, the admin and (step by step) the viewer: square, ink/paper, IBM Plex
Mono, 2px structural borders, cyan only as a fill. Follows DESIGN.md Part 1 and the "Admin UI patterns" in
agents/PLATFORM.md. Svelte 5 runes, snippets and callback props (no `<slot>`, no event dispatchers).

Styled only with `@scutoons/tokens` custom properties, so the app must `import '@scutoons/tokens'` once. No app
state, no routing, no paraglide: every visible string is a prop (defaults are English, e.g. `closeLabel = 'Close'`).

## Public API (`import … from '@scutoons/ui'`)

| Export | Props / usage |
|---|---|
| `Button` | `variant` `primary` \| `secondary` \| `clear`, `size` `md` \| `sm` (dense tools), `href` (renders `<a>`), `disabled`, any button/anchor attribute; children = label |
| `IconButton` | `label` (required: aria-label + title), `icon` (image URL, 16px) or children, `pressed`; 2rem circle, cyan fill on hover/pressed |
| `Field` | `label`, `hint`, `error`, `required`, `id`; `children({ id, describedby, invalid })` renders the app's input/select/textarea (styled inside `.field`) |
| `Panel` | framed box: `title`, `hint`, `actions` snippet, `quiet` (surface-panel, no border) |
| `Section` | entity-detail section: `title`, `hint`, `actions` snippet |
| `Sheet` | modal on native `<dialog>`: `bind:open`, `title`, `side` `center` \| `bottom` \| `right`, `closeLabel`, `onclose`, `footer` snippet (sticky action bar) |
| `Tabs` | `tabs: { id, label, disabled? }[]`, `bind:value`, `label`, `onchange`, `children(activeId)` = active panel; arrow keys / Home / End |
| `Toggle` | switch: `bind:checked`, `label`, `hint`, `disabled`, `onchange(checked)` |
| `Badge` | `tone` `neutral` \| `ink` \| `accent` \| `warning` \| `error`; children = text |
| `Menu` | dropdown: `label`, `trigger` snippet, `align` `start` \| `end`, `bind:open`; children = the items (buttons / links); closes on outside click, Escape, item click |
| `createToaster({ timeout?, max? })` | runes store: `items`, `addToast({ message, type?, timeout? })`, `dismiss(id)`, `clear()`; errors stay until dismissed |
| `ToastStack` | `toaster`, `dismissLabel`; fixed bottom-right |
| `clickOutside(handler, { escape? })` | attachment: `<div {@attach clickOutside(close)}>` |
| `DataTable` | generic (`T`): `columns: Column[]` (`{ id, label, sortable?, align?, width?, secondary? }` – secondary hides < 768px), `rows`, `rowKey`, `cell(row, column)` snippet, `sort` (`field`\|`-field`) + `sortHref(sort)` (URL-driven) or `onsort`, `defaultSortDir`, `selectable` + `bind:selected` + `bulk(keys)` snippet (bar above the table), `onrowclick` (click / Enter), `activeKey`, `rowClass`, `loading`, `empty` / `emptyLabel`, `caption`, `labels`; `nextSort(current, field)` |
| `Pager` | URL-driven: `page`, `pageSize`, `total`, `href(page)`, optional `sizeHref(size)` + `pageSizes`, `labels` |
| `SearchField` | debounced search input: `label`, `value` (follows the URL), `onsearch(q)`, `delay` |
| `readList(url, { pageSize, sort, filters })` · `listHref(url, patch)` · `listParams(state)` | pure URL helpers for `?page=&page_size=&sort=&q=&<filter>=` lists (a change resets `page`) |
| `SplitView` | list + detail: `open`, `title`, `onclose`, `list` / `detail` / `actions` snippets; ≥ `breakpoint` (1100px) a sticky pane (`--split-view-top`), narrower a right drawer (Sheet) |
| `CommandPalette` | `bind:open`, `commands: Command[]` (`{ id, label, hint?, group?, keywords?, run }`), `dynamic(query)` (query-dependent commands first), `hotkeys` (Ctrl/⌘+K, `/` outside text fields), labels; `matchCommand` |
| `Skeleton` | loading placeholder: `variant` `text` (`lines`) \| `block` (`height`) \| `table` (`rows`, `columns`), `label`; static bars, no shimmer |
| `Chart` | plain SVG `kind` `line` \| `bar`, `labels` (x), `series: { label, values }[]` (≤ 4), `height`, `ariaLabel`, `formatLabel`, `formatValue`; one y-axis from 0, cyan only as fill, series told apart by fill/outline/hatch or dash pattern + legend; hover / arrow keys = crosshair + values |
| `RecordInspector` | raw record as key/value table or JSON (toggle): `record`, `labels` |

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
