<script lang="ts">
	let {
		text,
		size = 2,
		align = 'center',
		rotation = 0
	}: {
		text: string[];
		size?: number;
		align?: 'left' | 'right' | 'center';
		rotation?: number;
	} = $props();
</script>

<div
	class="onomatopeia-thorns"
	style="font-size: {size}rem; text-align: {align}; transform: rotate({rotation}deg)"
>
	{#each text as line}
		<div class="line">{line}</div>
	{/each}
</div>

<style>
	.onomatopeia-thorns {
		font-weight: bold;
		text-transform: uppercase;
		line-height: 1.2;
		white-space: nowrap;
		position: relative;
		filter: drop-shadow(0 0 3px currentColor);
	}

	.onomatopeia-thorns::before,
	.onomatopeia-thorns::after {
		content: '';
		position: absolute;
		width: 100%;
		height: 100%;
		top: 0;
		left: 0;
	}

	.onomatopeia-thorns::before {
		transform: scaleX(1.1) scaleY(0.9);
		opacity: 0.5;
	}

	.onomatopeia-thorns::after {
		transform: scaleX(0.9) scaleY(1.1);
		opacity: 0.5;
	}

	.line {
		display: block;
		position: relative;
		z-index: 1;
	}
</style>
