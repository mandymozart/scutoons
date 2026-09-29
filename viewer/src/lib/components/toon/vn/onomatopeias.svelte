<script lang="ts">
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte.ts';
	import { Onomatopoeia } from '@scutoons/reader';

	const vn = getVisualNovelContext();

	const isActive = $derived(vn.state.activeOnomatopoeia.length > 0);

	function handleClick() {
		const hasWaitingOnomatopoeia = vn.state.activeOnomatopoeia.some((o) => o.wait);
		if (hasWaitingOnomatopoeia) {
			vn.continueFromOnomatopoeia();
		}
	}
</script>

<div
	class="onomatopeias"
	class:active={isActive}
	onclick={handleClick}
	onkeydown={handleClick}
	role="button"
	tabindex="0"
>
	{#each vn.state.activeOnomatopoeia as onomatopoeia (onomatopoeia.id)}
		<Onomatopoeia {onomatopoeia} />
	{/each}
</div>

<style>
	.onomatopeias {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
		z-index: 50;
		/* background: radial-gradient(var(--color-acid-purple), transparent); */
	}

	.onomatopeias.active {
		pointer-events: auto;
		cursor: pointer;
	}
</style>
