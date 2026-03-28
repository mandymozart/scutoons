<script lang="ts">
	import { asset } from '$lib/utils/cdn.ts';

	let { collectable } = $props();
</script>

<div class="collectable">
	<img src={asset(collectable.image)} alt={collectable.name} title={collectable.name} />
</div>

<style>
	.collectable {
		display: flex;
		align-items: center;
		flex-direction: column;
		justify-content: center;
		transform: scale(1);
		transition: transform 1s
			linear(
				0,
				0.03 1.1%,
				0.125 2.4%,
				0.906 9.8%,
				1.046 12.3%,
				1.11 15%,
				1.116 16.3%,
				1.11 17.8%,
				1.014 25.8%,
				0.987 31.2%,
				1.001 47.2%,
				1
			);
		&:hover {
			transform: scale(1.1);
		}
	}
	img {
		width: 100%;
	}
</style>
