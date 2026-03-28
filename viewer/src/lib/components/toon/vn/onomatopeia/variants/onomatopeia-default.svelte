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
	class="onomatopeia-default"
	style="font-size: {size}rem; text-align: {align}; transform: rotate({rotation}deg)"
>
	{#each text as line}
		<div class="line">{line}</div>
	{/each}
</div>

<style>
	.onomatopeia-default {
		font-weight: bold;
		text-transform: uppercase;
		line-height: 1.2;
		white-space: nowrap;
	}

	.line {
		display: block;
	}
</style>