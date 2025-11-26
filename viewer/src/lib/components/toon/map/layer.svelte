<script lang="ts">
	import { page } from '$app/state';
	import { getNavigationContext } from '$lib/contexts/navigation.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';

	let {
		key,
		width,
		height,
		image
	}: {
		key: string;
		width: number;
		height: number;
		image: string;
	} = $props();

	let nav = getNavigationContext();

	let mapVisible = $state(false);
	$effect(() => {
		if (nav.isOpen('map')) {
			mapVisible = true;
		}
	});
	$inspect(image);
</script>

<div class="layer layer--{key}" class:map-visible={mapVisible}>
	<img src={asset(`${page.params.toonId}/${image}`)} alt="Map {key}" {width} {height} />
</div>

<style>
	.layer {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}

	.layer img {
		display: block;
		width: 100%;
		height: 100%;
		opacity: 0;
		transition: opacity 1s
			linear(
				0,
				0.021 9%,
				0.152 14.4%,
				0.993 24.2%,
				0.078 34%,
				0.998 42%,
				0.196 48.4%,
				0.993 54.2%,
				0.329 59.2%,
				0.994 64.2%,
				0.471 68.4%,
				0.993 72.8%,
				0.614 76.4%,
				0.995 80.4%,
				0.751 83.6%,
				0.996 87.4%,
				0.875 90.2%,
				0.995 93.8%,
				0.968 96%,
				1
			);
	}

	.map-visible {
		img {
			opacity: 1;
		}
	}
</style>
