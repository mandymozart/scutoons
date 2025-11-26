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
	class="onomatopeia-outline"
	style="font-size: {size}rem; text-align: {align}; transform: rotate({rotation}deg)"
>
	{#each text as line}
		<div class="line">{line}</div>
	{/each}
</div>

<style>
	.onomatopeia-outline {
		font-weight: bold;
		text-transform: uppercase;
		line-height: 1.2;
		white-space: nowrap;
		text-shadow:
			-1px -1px 0 white,
			0 -1px 0 white,
			1px -1px 0 white,
			1px 0 0 white,
			1px 1px 0 white,
			0 1px 0 white,
			-1px 1px 0 white,
			-1px 0 0 white;
	}

	.line {
		display: block;
	}
</style>
