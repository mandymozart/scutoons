<script lang="ts">
	import { CloseButton } from '@scutoons/ui';
	import HomePanel from './home-panel.svelte';
	import HomeRoom from './home-room.svelte';
	import {
		DEFAULT_HOME_LABELS,
		type HomeLabels,
		type HomePiece,
		type HomeRecipe,
		type HomeSceneFiles,
		type HomeThing
	} from './home-types.ts';

	/**
	 * The home view (agents/research/CAVE.md D3–D6): full screen above the app (owner 2026-10-14: "the cave design is a
	 * full screen i will need this later in the live scenario as well") – a bar (where you are, close), the room, the
	 * panel (phone: below the room, scrolling; desktop: at the side). In the reader's own home things move: select
	 * something carried, then tap a rack (put in) or the room (place a piece of furniture – it stays in this home); drag a
	 * piece to move it. A visitor (a friend's visit link) only looks – and uses the workbench. Escape closes.
	 */
	let {
		scene,
		avatar = null,
		furniture,
		carried,
		recipes = [],
		where,
		visitor = false,
		note = '',
		labels = DEFAULT_HOME_LABELS,
		fits,
		onplace,
		onmove,
		onputin,
		ontakeout,
		oncraft,
		onclose
	}: {
		scene: HomeSceneFiles;
		avatar?: string | null;
		furniture: HomePiece[];
		carried: HomeThing[];
		/** the workbench's recipes (when the home has one) */
		recipes?: HomeRecipe[];
		/** the bar's text: whose home, where */
		where: string;
		visitor?: boolean;
		/** the app's answer to the last action (a refusal, a success) */
		note?: string;
		labels?: HomeLabels;
		/** does a carried thing fit into a piece (its `holds`) */
		fits: (thing: HomeThing, piece: HomePiece) => boolean;
		onplace: (thing: HomeThing, x: number, y: number) => void;
		onmove: (piece: HomePiece, x: number, y: number, flip: boolean) => void;
		onputin: (thing: HomeThing, piece: HomePiece) => void;
		ontakeout: (thing: HomeThing, piece: HomePiece) => void;
		oncraft: (recipe: HomeRecipe) => void;
		onclose: () => void;
	} = $props();

	let selected = $state<string | null>(null);
	let openId = $state<string | null>(null);
	const sel = $derived(carried.find((c) => c.id === selected) ?? null);
	const open = $derived(furniture.find((p) => p.id === openId) ?? null);
	const targets = $derived(
		sel && !visitor ? furniture.filter((p) => p.holds && fits(sel, p)).map((p) => p.id) : []
	);

	function tap(piece: HomePiece) {
		if (sel && !visitor && piece.holds && fits(sel, piece)) {
			onputin(sel, piece);
			selected = null;
		}
		openId = piece.id;
	}

	function place(x: number, y: number) {
		if (!sel?.furniture || visitor) return;
		onplace(sel, x, y);
		openId = sel.id;
		selected = null;
	}

	function keydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onclose();
	}
</script>

<svelte:window onkeydown={keydown} />

<div class="home-view" role="dialog" aria-modal="true" aria-label={where} data-testid="home-view">
	<header>
		<span data-testid="home-where">{where}</span>
		<CloseButton
			label={visitor ? labels.visitLeave : labels.leave}
			onclick={onclose}
			data-testid="home-leave"
		/>
	</header>
	<div class="body">
		<div class="stage">
			<HomeRoom
				{scene}
				{avatar}
				{furniture}
				open={openId}
				{targets}
				editable={!visitor}
				placing={!!sel?.furniture && !visitor}
				{labels}
				ontap={tap}
				onroom={place}
				onmove={(piece, x, y) => onmove(piece, x, y, !!piece.flip)}
			/>
		</div>
		<HomePanel
			{open}
			{carried}
			{recipes}
			{selected}
			{visitor}
			{note}
			{labels}
			onselect={(thing) => (selected = thing?.id ?? null)}
			{ontakeout}
			onturn={(piece) => onmove(piece, piece.x, piece.y, !piece.flip)}
			{oncraft}
		/>
	</div>
</div>

<style>
	/* full screen above the app, like the ScoreView */
	.home-view {
		position: fixed;
		inset: 0;
		z-index: var(--home-view-z, 2080);
		display: grid;
		grid-template-rows: auto minmax(0, 1fr);
		background: var(--paper);
		color: var(--ink);
	}
	header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0 0.5rem 0 1rem;
		min-height: var(--control-height);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	/* phone: the room on top, the panel scrolls below; desktop: the room fills the height, the panel at the side */
	.body {
		display: grid;
		grid-template-rows: auto minmax(0, 1fr);
		min-height: 0;
	}
	.stage {
		display: grid;
		place-items: center;
		padding: 0.5rem;
	}
	.stage > :global(.room) {
		width: min(100%, 55vh);
	}
	@media (min-width: 64rem) {
		.body {
			grid-template-rows: none;
			grid-template-columns: minmax(0, 1fr) 22rem;
		}
		.stage > :global(.room) {
			width: min(100%, 100vh - 5rem);
		}
		.body > :global(.panel) {
			padding-top: 1rem;
			border-left: var(--border-width-regular) solid var(--ink);
		}
	}
</style>
