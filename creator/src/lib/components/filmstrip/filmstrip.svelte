<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { frameWidth, groupOpen, type FilmFrame, type FilmGroup, type FilmKeys } from '$lib/editor/filmstrip.ts';

	/**
	 * The Filmstrip (lap 2 C3): the open item's parts in order – a storyboard's shots, a chapter's slices, an experience's
	 * scenes – in groups under a head that folds, each part a frame in its own shape with its name, a line under it and a
	 * value at its end. A press makes a part the current one; Ctrl / ⌘ and Shift hand their keys to `onpick` (the page
	 * picks). The current part takes the list's selected ground, a picked one the text colour's outline (no side bar,
	 * RULES H10). `row` lays the parts side by side (a strip along the bottom).
	 */
	let {
		groups,
		current,
		picked = [],
		onpick,
		label,
		hint,
		row = false,
		empty,
		testid = 'filmstrip'
	}: {
		groups: readonly FilmGroup[];
		current: string | null;
		picked?: readonly string[];
		onpick: (id: string, keys: FilmKeys) => void;
		label: string;
		/** how picking works, as hover text */
		hint?: string;
		row?: boolean;
		/** the hover text of a frame without a picture */
		empty?: string;
		testid?: string;
	} = $props();

	/** the heads the member folded or opened by hand */
	let toggled = $state<Record<string, boolean>>({});
	const set = $derived(new Set(picked));
	/** one group without a head: only its parts */
	const bare = $derived(groups.length === 1 && !groups[0].label && !groups[0].number);
	const FRAME_REM = 2.9;

	function fold(group: FilmGroup) {
		toggled = { ...toggled, [group.key]: !groupOpen(group, current, toggled) };
	}
	// the current part scrolls into view when it changes from elsewhere (← →, a link)
	let list: HTMLElement | null = $state(null);
	$effect(() => {
		const id = current;
		if (!id || !list) return;
		const el = list.querySelector<HTMLElement>(`[data-frame="${CSS.escape(id)}"]`);
		el?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
	});
</script>

{#snippet frame(f: FilmFrame)}
	<button
		type="button"
		class="part"
		class:current={f.id === current}
		class:picked={set.has(f.id) && f.id !== current}
		aria-current={f.id === current ? 'true' : undefined}
		aria-pressed={set.has(f.id) || f.id === current}
		data-frame={f.id}
		data-control-exempt="a frame of the filmstrip"
		data-testid="{testid}-{f.id}"
		title={f.title}
		onclick={(e) => onpick(f.id, { shiftKey: e.shiftKey, ctrlKey: e.ctrlKey, metaKey: e.metaKey })}
	>
		<span class="frame" style:width="{row ? undefined : frameWidth(f.aspect, FRAME_REM)}rem" style:aspect-ratio={row ? `${f.aspect}` : undefined} title={f.src ? undefined : empty}>
			{#if f.src}<img src={f.src} alt="" loading="lazy" />{/if}
		</span>
		<span class="words">
			<span class="name">{f.label}</span>
			{#if f.detail}<span class="detail">{f.detail}</span>{/if}
		</span>
		{#if f.meta}<span class="meta">{f.meta}</span>{/if}
	</button>
{/snippet}

<nav class="filmstrip" class:row aria-label={label} title={hint} bind:this={list} data-testid={testid}>
	{#each groups as g (g.key)}
		{@const open = bare || groupOpen(g, current, toggled)}
		{#if !bare}
			<button type="button" class="head" aria-expanded={open} data-control-exempt="a group's head in the filmstrip" onclick={() => fold(g)} data-testid="{testid}-head-{g.key}">
				<Icon name={open ? 'chevron-down' : 'chevron-right'} size="0.9rem" />
				{#if g.number}<span class="num">{g.number}</span>{/if}
				<span class="title">{g.label}</span>
				<span class="count">{g.frames.length}</span>
			</button>
		{/if}
		{#if open}
			<div class="parts" role="group" aria-label={[g.number, g.label].filter(Boolean).join(' ')}>
				{#each g.frames as f (f.id)}{@render frame(f)}{/each}
			</div>
		{/if}
	{/each}
</nav>

<style>
	.filmstrip {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
		padding: 0 0 var(--editor-pad);
		box-sizing: border-box;
		user-select: none;
	}
	.head,
	.part {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		width: 100%;
		margin: 0;
		border: 0;
		border-radius: var(--editor-radius);
		background: var(--kit-list-row-rest-ground);
		color: var(--kit-list-row-rest-text);
		font: inherit;
		text-align: left;
		cursor: pointer;
		box-sizing: border-box;
	}
	.head {
		min-height: var(--editor-tree-row);
		padding: 0 calc(var(--editor-pad) / 2);
		gap: 0.35rem;
	}
	.head > :global(svg) {
		flex: none;
		color: var(--tool-muted);
	}
	.num,
	.name {
		font-family: var(--font-mono);
	}
	.num {
		flex: none;
	}
	.title {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--editor-small);
	}
	.count,
	.meta,
	.detail {
		flex: none;
		color: var(--tool-muted);
		font-size: var(--editor-small);
		white-space: nowrap;
	}
	.parts {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}
	.part {
		min-height: 3.4rem;
		padding: 0.25rem calc(var(--editor-pad) / 2) 0.25rem 1.1rem;
	}
	@media (hover: hover) {
		.head:hover,
		.part:hover:not(.current) {
			background: var(--kit-list-row-hover-ground);
		}
	}
	.part.current {
		background: var(--kit-list-row-selected-ground);
		color: var(--kit-list-row-selected-text);
	}
	.part.current .detail,
	.part.current .meta {
		color: var(--kit-list-row-selected-text);
	}
	/* picked beside the current one: the text colour's outline all round (RULES H10: no side bar) */
	.part.picked {
		box-shadow: inset 0 0 0 var(--editor-edge) var(--tool-text);
	}
	.head:focus-visible,
	.part:focus-visible {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: -2px;
	}
	.frame {
		flex: none;
		display: block;
		height: 2.9rem;
		border: var(--editor-edge) solid var(--kit-thumb-rest-border);
		border-radius: 0.15rem;
		overflow: hidden;
		box-sizing: border-box;
		background: repeating-linear-gradient(135deg, transparent 0 0.25rem, color-mix(in srgb, var(--tool-text) 9%, transparent) 0.25rem 0.3rem);
	}
	.frame img {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: cover;
		object-position: top;
	}
	.words {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		gap: 0.1rem;
	}
	.name,
	.detail {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.name {
		font-size: var(--editor-small);
	}
	/* side by side: the parts along one line, each frame as high as the strip allows, its words under it */
	.row {
		flex-direction: row;
		height: 100%;
		overflow-x: auto;
		padding: 0 0 0.25rem;
	}
	.row .parts {
		flex-direction: row;
		gap: 0.5rem;
		height: 100%;
	}
	.row .part {
		flex-direction: column;
		align-items: stretch;
		width: auto;
		height: 100%;
		min-height: 5rem;
		padding: 0.25rem;
	}
	.row .frame {
		flex: 1;
		height: auto;
		min-height: 0;
	}
	.row .meta {
		display: none;
	}
</style>
