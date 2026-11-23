<script lang="ts">
	import { onDestroy, onMount, type Snippet } from 'svelte';
	import Icon from '../icon.svelte';
	import CloseButton from '../close-button.svelte';
	import type { Floats } from './floats.svelte.ts';
	import type { Edge } from './floats.ts';
	import { measureDrop } from './measure-drop.ts';

	/**
	 * One floating panel over the stage: its head is the panel's own surface with its tabs and – unless essential – the
	 * close; the whole head drags the panel, a tab name dragged moves that tab (into another panel's head, a marker shows
	 * where; to a screen edge or corner its view takes the stage; elsewhere it becomes a panel of its own). A double-click on the head (or Enter on its keyboard handle)
	 * folds and unfolds it. Open, the edges resize it – over its neighbours, never pushing them. An inline panel (a
	 * toolbar) is one row or one column. The page lays the panels out in `floats`; this draws one. `children` = the front
	 * tab's content (or the toolbar's row). A view on the stage or over everything is not among its tabs.
	 */
	let {
		floats,
		id,
		children,
		label,
		closeLabel = 'Close',
		moveLabel = 'Move the panel – drag the head, or the arrow keys; Enter or a double-click folds it'
	}: {
		floats: Floats;
		id: string;
		children: Snippet<[tab: string | undefined]>;
		label?: string;
		closeLabel?: string;
		moveLabel?: string;
	} = $props();

	const spec = $derived(floats.spec(id)!);
	const view = $derived(floats.views[id]);
	const collapsed = $derived(view?.collapsed ?? false);
	const folded = $derived(collapsed);
	const vertical = $derived(floats.vertical(id));
	const held = $derived(floats.held?.id === id && floats.held.moved);
	const tabs = $derived(floats.tabsOf(id));
	const active = $derived(floats.activeOf(id));
	const contentSized = $derived(spec?.size.w === 'content' || spec?.size.h === 'content');
	const draggedTab = $derived(floats.heldTab?.moved ? floats.heldTab.tab : null);
	/** flung off the screen: it flies on from where it was let go */
	const thrown = $derived(floats.thrown?.id === id ? floats.thrown : null);
	const rect = $derived(thrown?.rect ?? view?.rect ?? { x: 0, y: 0, w: 0, h: 0 });
	const rem = (px: number) => `${px / (floats.metrics.rem || 16)}rem`;

	onMount(() => floats.show(id));
	onDestroy(() => floats.hide(id));

	const press = (event: PointerEvent, edge: Edge | null = null) => {
		if (event.button !== 0) return;
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
		floats.grab(id, event.clientX, event.clientY, edge);
		event.preventDefault();
	};
	const move = (event: PointerEvent) => floats.drag(event.clientX, event.clientY);
	const release = () => floats.drop();
	/** The whole head is the handle: a press anywhere but on a tab name or a button moves the panel */
	const pressHead = (event: PointerEvent) => {
		if ((event.target as Element).closest('button, [data-tab]')) return;
		press(event);
	};
	/** A double-click on the head, its tab names too, folds or unfolds the panel */
	const foldHead = (event: MouseEvent) => {
		if ((event.target as Element).closest('button:not([data-tab])')) return;
		floats.setCollapsed(id, !collapsed);
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
		floats.grab(id, r.x, r.y, null, false);
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
		class:thrown
		aria-label={label ?? spec.name}
		data-float={id}
		data-tabbed={spec.inline ? undefined : ''}
		style:left="{rect.x - floats.area.x + floats.metrics.gap}px"
		style:top="{rect.y - floats.area.y + floats.metrics.gap}px"
		style:width={spec.inline && vertical ? undefined : `${rect.w}px`}
		style:min-width={spec.inline && vertical ? `${rect.w}px` : undefined}
		style:height={spec.inline && !vertical ? undefined : `${rect.h}px`}
		style:min-height={spec.inline && !vertical ? `${rect.h}px` : undefined}
		style:z-index={floats.z(id) + (held || thrown ? 100 : 0)}
		style:--throw-x={thrown ? rem(thrown.x) : undefined}
		style:--throw-y={thrown ? rem(thrown.y) : undefined}
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
				ondblclick={foldHead}
			>
				<!-- the keyboard's handle: no glyph, the head's leading room -->
				<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
				<div class="handle" role="button" tabindex="0" aria-label="{spec.name}: {moveLabel}" title={spec.about} onkeydown={handleKey}></div>
				<div class="strip" role="tablist" aria-label={spec.name} data-tab-strip>
					{#each folded ? tabs.filter((t) => t === active) : tabs as tab, i (tab)}
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
							><span class="label">{t?.name ?? tab}</span></button
						>
					{/each}
				</div>
				{#if !spec.essential}
					<span class="buttons">
						<CloseButton label={closeLabel} stroke={1} class="panel-close" onclick={() => floats.close(id)} />
					</span>
				{/if}
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
		border: var(--editor-edge) solid var(--kit-panel-rest-border);
		/* twice a control's corner (floats.rs); what sits in its head is rounded concentric with it (the head's inset less) */
		border-radius: var(--kit-radius-box);
		/* the deck editor's floating panel (deck-kit floats.rs, RULES H11): the surface at the panel opacity, see-through, no
		   blur, no shadow; its text and icons carry the halo (the ink ring of the ground's colour) so they read over any
		   stage. Controls keep their own opaque ground (no halo on them, as the deck's DrawList::halo) */
		background: var(--kit-panel-rest-ground);
		color: var(--tool-text);
		text-shadow: var(--tool-halo);
		font-family: var(--editor-sans);
		/* content in a panel stands on the panel's ground: a component's "ground" (or the shared --paper) is see-through here */
		--tool-ground: transparent;
		--paper: transparent;
		/* folding and unfolding (a double-click), a keyboard move: the panel eases into its new rectangle */
		transition:
			left var(--motion-base) var(--motion-ease),
			top var(--motion-base) var(--motion-ease),
			height var(--motion-base) var(--motion-ease),
			width var(--motion-base) var(--motion-ease),
			border-radius var(--motion-base) var(--motion-ease);
	}
	.float.held {
		transition: none;
	}
	/* flung off the screen: on out along the throw, fading */
	.float.thrown {
		pointer-events: none;
		animation: throw-out var(--motion-slow) var(--motion-ease-in) forwards;
	}
	@keyframes throw-out {
		to {
			transform: translate(var(--throw-x), var(--throw-y));
			opacity: 0;
		}
	}
	.float :global(svg:not(:is([role='menu'], [role='listbox'], [role='dialog'], [popover]) svg)) {
		filter: var(--tool-icon-halo);
	}
	.head,
	.line {
		/* what sits in the head is rounded concentric with the panel (its corner less the head's inset) */
		--kit-r: calc(var(--kit-radius-box) - 0.1875rem);
		flex: none;
		display: flex;
		align-items: center;
		gap: 0.125rem;
		min-width: 0;
		overflow: hidden;
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
		color: var(--tool-line);
		outline: none;
	}
	.head .handle {
		width: 0.375rem;
	}
	.handle:focus-visible {
		box-shadow: inset 0 0 0 2px var(--kit-focus-rest-ring);
	}
	.held .head,
	.held .line {
		cursor: grabbing;
	}
	/* the grip: two columns of dots */
	.grip {
		width: 0.375rem;
		height: 0.875rem;
		background: radial-gradient(circle, var(--tool-muted) 1px, transparent 1.5px) 0 0 / 0.1875rem 0.21875rem;
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
	/* a small panel: every tab name shrinks and ends with an ellipsis, nothing wraps or spills out of the head */
	.tab {
		flex: 0 1 auto;
		display: inline-flex;
		align-items: center;
		min-width: 1.75rem;
		height: calc(var(--control-height) - 0.25rem);
		padding: 0 var(--editor-pad);
		border: 0;
		border-radius: var(--kit-r);
		background: none;
		color: var(--tool-muted);
		font: inherit;
		font-size: var(--editor-small);
		white-space: nowrap;
		overflow: hidden;
		text-shadow: inherit;
		cursor: pointer;
		touch-action: none;
	}
	.label {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	/* RULES H4: no border at rest – the front tab in the text colour, the others muted; a rounded border on hover and while
	   dragged (kit.css --kit-tab-*) */
	.tab.on {
		color: var(--kit-tab-on-text);
	}
	.tab.dragged {
		box-shadow: inset 0 0 0 var(--editor-edge) var(--kit-tab-dragged-border);
		background: var(--kit-tab-dragged-ground);
		color: var(--kit-tab-dragged-text);
		text-shadow: none;
	}
	.tab:focus-visible,
	.icon:focus-visible {
		outline: 2px solid var(--kit-focus-rest-ring);
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
		border-radius: var(--kit-r);
		background: none;
		color: var(--tool-muted);
		cursor: pointer;
	}
	.buttons :global(.panel-close) {
		border-radius: var(--kit-r);
		color: var(--tool-muted);
	}
	@media (hover: hover) {
		.tab:hover {
			box-shadow: inset 0 0 0 var(--editor-edge) var(--kit-tab-hover-border);
			color: var(--kit-tab-hover-text);
		}
		.icon:hover,
		.buttons :global(.panel-close:hover) {
			color: var(--tool-text);
			background: var(--kit-icon-button-hover-ground);
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
		animation: unfold var(--motion-base) var(--motion-ease);
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		overflow: auto;
	}
	@keyframes unfold {
		from {
			opacity: 0;
		}
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
