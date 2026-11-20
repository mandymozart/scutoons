<script lang="ts" module>
	import { dropTarget, markerX, type DropPanel, type DropTarget } from './panel-tabs.ts';
	import type { Rect } from './floats.ts';

	const box = (el: Element): Rect => {
		const r = el.getBoundingClientRect();
		return { x: r.left, y: r.top, w: r.width, h: r.height };
	};

	/** The tab-holding panels under the pointer, front to back, measured from the page, and where a tab would land */
	function measureDrop(floats: Floats, x: number, y: number): { target: DropTarget; marker: Rect | null } {
		const panels: DropPanel[] = [];
		for (const el of document.querySelectorAll<HTMLElement>('[data-float][data-tabbed]')) {
			const id = el.dataset.float!;
			const strip = el.querySelector('[data-tab-strip]');
			if (!strip) continue;
			panels.push({ id, rect: box(el), strip: box(strip), tabs: [...strip.querySelectorAll('[data-tab]')].map(box) });
		}
		panels.sort((a, b) => floats.z(b.id) - floats.z(a.id));
		const target = dropTarget(x, y, panels);
		const p = target && panels.find((o) => o.id === target.panel);
		if (!target || !p) return { target, marker: null };
		return { target, marker: { x: markerX(p.tabs, p.strip, target.index) - 1, y: p.strip.y, w: 2, h: p.strip.h } };
	}
</script>

<script lang="ts">
	import { onDestroy, onMount, type Snippet } from 'svelte';
	import Icon from '../icon.svelte';
	import CloseButton from '../close-button.svelte';
	import type { Floats } from './floats.svelte.ts';
	import type { Edge } from './floats.ts';

	/**
	 * One floating panel over the stage: no title bar – its head is a drag handle (the grip and the free room) with the
	 * panel's tabs, then fold and, unless essential, close. A double-click on the head or Enter on the grip folds it; a
	 * tab dragged into another panel's head moves there (a marker shows where), onto the stage it becomes a panel of its
	 * own. Open, the edges resize it and push the neighbours. An inline panel (a toolbar) is one row or one column. The
	 * page lays the panels out in `floats`; this draws one. `children` = the front tab's content (or the toolbar's row).
	 */
	let {
		floats,
		id,
		children,
		label,
		foldLabel = 'Fold',
		openLabel = 'Open',
		closeLabel = 'Close',
		moveLabel = 'Move the panel – drag, or the arrow keys; Enter folds it'
	}: {
		floats: Floats;
		id: string;
		children: Snippet<[tab: string | undefined]>;
		label?: string;
		foldLabel?: string;
		openLabel?: string;
		closeLabel?: string;
		moveLabel?: string;
	} = $props();

	const spec = $derived(floats.spec(id)!);
	const view = $derived(floats.views[id]);
	const collapsed = $derived(view?.collapsed ?? false);
	const vertical = $derived(floats.vertical(id));
	const held = $derived(floats.held?.id === id && floats.held.moved);
	const tabs = $derived(floats.tabsOf(id));
	const active = $derived(floats.activeOf(id));
	const contentSized = $derived(spec?.size.w === 'content' || spec?.size.h === 'content');
	const draggedTab = $derived(floats.heldTab?.moved ? floats.heldTab.tab : null);

	onMount(() => floats.show(id));
	onDestroy(() => floats.hide(id));

	const press = (event: PointerEvent, edge: Edge | null = null) => {
		if (event.button !== 0) return;
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
		floats.grab(id, event.clientX, event.clientY, edge);
		event.preventDefault();
	};
	const move = (event: PointerEvent) => floats.drag(event.clientX, event.clientY);
	const release = (open = false) => {
		const dragged = floats.drop();
		if (open && !dragged) floats.setCollapsed(id, false);
	};
	/** The head's free room is a handle too: a press there (not on a tab or a button) moves the panel */
	const pressHead = (event: PointerEvent) => {
		if ((event.target as Element).closest('button, [data-tab]')) return;
		press(event);
	};

	const handleKey = (event: KeyboardEvent) => {
		if (event.key === 'Enter' && !spec.inline) {
			event.preventDefault();
			floats.setCollapsed(id, !collapsed);
			return;
		}
		const step = event.shiftKey ? 64 : 16;
		const d = ({ ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] } as Record<string, number[]>)[event.key];
		if (!d || !view) return;
		event.preventDefault();
		event.stopPropagation();
		const r = view.rect;
		floats.grab(id, r.x, r.y);
		floats.drag(r.x + d[0] * 2, r.y + d[1] * 2);
		floats.drag(r.x + d[0], r.y + d[1]);
		floats.drop();
	};

	// ── a tab in the hand ──
	const pressTab = (event: PointerEvent, tab: string) => {
		if (event.button !== 0) return;
		event.stopPropagation();
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
		floats.grabTab(tab, event.clientX, event.clientY);
	};
	const moveTab = (event: PointerEvent) => {
		if (!floats.heldTab) return;
		const { target, marker } = measureDrop(floats, event.clientX, event.clientY);
		floats.dragTab(event.clientX, event.clientY, target, marker);
	};
	const tabKey = (event: KeyboardEvent, index: number) => {
		const next = { ArrowLeft: index - 1, ArrowRight: index + 1, Home: 0, End: tabs.length - 1 }[event.key];
		if (next === undefined || !tabs[next]) return;
		event.preventDefault();
		event.stopPropagation();
		floats.select(id, tabs[next]);
		(event.currentTarget as HTMLElement).parentElement?.querySelectorAll<HTMLElement>('[data-tab]')[next]?.focus();
	};

	const EDGES: Array<{ edge: Edge; name: string }> = [
		{ edge: { x: 0, y: -1 }, name: 'n' },
		{ edge: { x: 0, y: 1 }, name: 's' },
		{ edge: { x: -1, y: 0 }, name: 'w' },
		{ edge: { x: 1, y: 0 }, name: 'e' },
		{ edge: { x: -1, y: -1 }, name: 'nw' },
		{ edge: { x: 1, y: -1 }, name: 'ne' },
		{ edge: { x: -1, y: 1 }, name: 'sw' },
		{ edge: { x: 1, y: 1 }, name: 'se' }
	];

	let content = $state<HTMLDivElement>();
	$effect(() => {
		const el = content;
		if (!el || !contentSized) return;
		const observer = new ResizeObserver(() =>
			floats.measure(id, vertical ? { w: el.offsetWidth, h: el.scrollHeight } : { w: el.scrollWidth, h: el.offsetHeight })
		);
		if (el.firstElementChild) observer.observe(el.firstElementChild);
		observer.observe(el);
		return () => observer.disconnect();
	});
</script>

{#if view && spec}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<section
		class="float"
		class:inline={spec.inline}
		class:vertical
		class:collapsed
		class:held
		aria-label={label ?? spec.name}
		data-float={id}
		data-tabbed={spec.inline ? undefined : ''}
		style:left="{view.rect.x - floats.area.x + floats.metrics.gap}px"
		style:top="{view.rect.y - floats.area.y + floats.metrics.gap}px"
		style:width={spec.inline && vertical ? undefined : `${view.rect.w}px`}
		style:min-width={spec.inline && vertical ? `${view.rect.w}px` : undefined}
		style:height={spec.inline && !vertical ? undefined : `${view.rect.h}px`}
		style:min-height={spec.inline && !vertical ? `${view.rect.h}px` : undefined}
		style:z-index={floats.z(id) + (held ? 100 : 0)}
		onpointerdowncapture={() => floats.front(id)}
	>
		{#if spec.inline}
			<div class="line">
				<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
				<div
					class="handle"
					role="button"
					tabindex="0"
					aria-label="{spec.name}: {moveLabel}"
					title={spec.about}
					onpointerdown={press}
					onpointermove={move}
					onpointerup={() => release()}
					onpointercancel={() => release()}
					onkeydown={handleKey}
				>
					<span class="grip" aria-hidden="true"></span>
				</div>
				<div class="content" bind:this={content}>{@render children(undefined)}</div>
				<span class="buttons">
					<button
						type="button"
						class="icon"
						aria-pressed={vertical}
						onclick={() => floats.setVertical(id, !vertical)}
						aria-label={vertical ? 'As a row' : 'As a column'}
						title={vertical ? 'As a row' : 'As a column'}
					>
						<span class="orient" class:turned={vertical}><Icon name="columns" size="1rem" strokeWidth={2} /></span>
					</button>
				</span>
			</div>
		{:else}
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="head"
				onpointerdown={pressHead}
				onpointermove={move}
				onpointerup={() => release()}
				onpointercancel={() => release()}
				ondblclick={(e) => {
					if (!(e.target as Element).closest('button, [data-tab]')) floats.setCollapsed(id, !collapsed);
				}}
			>
				<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
				<div class="handle" role="button" tabindex="0" aria-label="{spec.name}: {moveLabel}" title={spec.about} onkeydown={handleKey}>
					<span class="grip" aria-hidden="true"></span>
				</div>
				<div class="strip" role="tablist" aria-label={spec.name} data-tab-strip>
					{#each collapsed ? tabs.filter((t) => t === active) : tabs as tab, i (tab)}
						{@const t = floats.tab(tab)}
						<button
							type="button"
							class="tab"
							class:on={tab === active}
							class:dragged={tab === draggedTab}
							role="tab"
							aria-selected={tab === active}
							tabindex={tab === active ? 0 : -1}
							title={t?.about}
							data-tab={tab}
							onpointerdown={(e) => pressTab(e, tab)}
							onpointermove={moveTab}
							onpointerup={() => floats.dropTab()}
							onpointercancel={() => floats.cancelTab()}
							onkeydown={(e) => tabKey(e, i)}
							ondblclick={(e) => e.stopPropagation()}>{t?.name ?? tab}</button
						>
					{/each}
				</div>
				<span class="buttons">
					<button
						type="button"
						class="icon"
						aria-expanded={!collapsed}
						onclick={() => floats.setCollapsed(id, !collapsed)}
						aria-label={collapsed ? openLabel : foldLabel}
						title={collapsed ? openLabel : foldLabel}
					>
						<Icon name={collapsed ? 'chevron-down' : 'minus'} size="1rem" strokeWidth={2} />
					</button>
					{#if !spec.essential}
						<CloseButton label={closeLabel} stroke={1} class="panel-close" onclick={() => floats.close(id)} />
					{/if}
				</span>
			</div>
			{#if !collapsed}
				<div class="body" role="tabpanel">{@render children(active)}</div>
			{/if}
		{/if}
		{#if !collapsed}
			{#each EDGES as e (e.name)}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="edge {e.name}"
					onpointerdown={(event) => press(event, e.edge)}
					onpointermove={move}
					onpointerup={() => release()}
					onpointercancel={() => release()}
					ondblclick={() => floats.unsize(id)}
				></div>
			{/each}
		{/if}
	</section>
{/if}

<style>
	.float {
		position: absolute;
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		min-width: 0;
		border: var(--editor-edge) solid var(--editor-line);
		border-radius: var(--editor-radius);
		background: var(--editor-panel-ground);
		color: var(--editor-text);
		text-shadow: var(--editor-halo);
		font-family: var(--editor-sans);
	}
	.float.held {
		box-shadow: 0 0.25rem 1.5rem rgb(0 0 0 / 0.35);
	}
	.head,
	.line {
		flex: none;
		display: flex;
		align-items: center;
		gap: 0.125rem;
		min-width: 0;
		padding: 0.1875rem 0.1875rem 0.1875rem 0;
		cursor: grab;
		touch-action: none;
		user-select: none;
	}
	.collapsed {
		border-radius: 2rem;
	}
	.handle {
		flex: none;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 1rem;
		align-self: stretch;
		color: var(--editor-line);
		outline: none;
	}
	.handle:focus-visible {
		box-shadow: inset 0 0 0 2px var(--editor-accent);
	}
	.held .head,
	.held .line {
		cursor: grabbing;
	}
	/* the grip: two columns of dots */
	.grip {
		width: 0.375rem;
		height: 0.875rem;
		background: radial-gradient(circle, var(--editor-muted) 1px, transparent 1.5px) 0 0 / 0.1875rem 0.21875rem;
		opacity: 0.7;
	}
	.strip {
		flex: 0 1 auto;
		display: flex;
		align-items: center;
		gap: 0.125rem;
		min-width: 0;
		overflow: hidden;
	}
	.tab {
		flex: 0 1 auto;
		min-width: 2.5rem;
		height: var(--control-height);
		padding: 0 0.75rem;
		border: 0;
		border-radius: 1.2rem;
		background: none;
		color: var(--editor-muted);
		font: inherit;
		font-size: var(--editor-small);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		text-shadow: inherit;
		cursor: pointer;
		touch-action: none;
	}
	.tab.on {
		background: var(--editor-pressed);
		color: var(--editor-pressed-text);
		text-shadow: none;
	}
	.tab.dragged {
		opacity: 0.4;
	}
	.tab:focus-visible,
	.icon:focus-visible {
		outline: 2px solid var(--editor-accent);
		outline-offset: -2px;
	}
	.buttons {
		flex: none;
		display: inline-flex;
		align-items: center;
		margin-left: auto;
	}
	.icon {
		display: inline-grid;
		place-items: center;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		border: 0;
		border-radius: 50%;
		background: none;
		color: var(--editor-muted);
		cursor: pointer;
	}
	.buttons :global(.panel-close) {
		border-radius: 50%;
		color: var(--editor-muted);
	}
	@media (hover: hover) {
		.tab:not(.on):hover,
		.icon:hover,
		.buttons :global(.panel-close:hover) {
			color: var(--editor-text);
			background: var(--editor-hover);
		}
	}
	.vertical .line {
		flex-direction: column;
		padding: 0 0.1875rem 0.1875rem;
	}
	.vertical .handle {
		width: auto;
		height: 1rem;
		align-self: stretch;
	}
	.vertical .grip {
		transform: rotate(90deg);
	}
	.vertical .buttons {
		margin-left: 0;
		margin-top: auto;
	}
	.orient {
		display: inline-flex;
	}
	.orient.turned {
		transform: rotate(90deg);
	}
	.content {
		flex: 0 1 auto;
		min-width: 0;
		overflow: hidden;
		cursor: default;
	}
	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: auto;
	}
	.edge {
		position: absolute;
		z-index: 2;
		touch-action: none;
	}
	.n,
	.s {
		left: 0.5rem;
		right: 0.5rem;
		height: 0.5rem;
		cursor: ns-resize;
	}
	.n {
		top: -0.25rem;
	}
	.s {
		bottom: -0.25rem;
	}
	.w,
	.e {
		top: 0.5rem;
		bottom: 0.5rem;
		width: 0.5rem;
		cursor: ew-resize;
	}
	.w {
		left: -0.25rem;
	}
	.e {
		right: -0.25rem;
	}
	.nw,
	.ne,
	.sw,
	.se {
		width: 0.75rem;
		height: 0.75rem;
	}
	.nw {
		left: -0.25rem;
		top: -0.25rem;
		cursor: nwse-resize;
	}
	.se {
		right: -0.25rem;
		bottom: -0.25rem;
		cursor: nwse-resize;
	}
	.ne {
		right: -0.25rem;
		top: -0.25rem;
		cursor: nesw-resize;
	}
	.sw {
		left: -0.25rem;
		bottom: -0.25rem;
		cursor: nesw-resize;
	}
</style>
