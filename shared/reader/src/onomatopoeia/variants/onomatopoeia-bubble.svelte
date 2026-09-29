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
	class="onomatopeia-bubble"
	style="font-size: {size}rem; text-align: {align}; transform: rotate({rotation}deg)"
>
	{#each text as line}
		<div class="line">{line}</div>
	{/each}
</div>

<style>
	.onomatopeia-bubble {
		font-weight: bold;
		text-transform: uppercase;
		line-height: 1.2;
		white-space: nowrap;
		padding: 0.5em 1em;
		background: white;
		border: 3px solid currentColor;
		border-radius: 50%;
		display: inline-block;
	}

	.line {
		display: block;
	}
</style>
