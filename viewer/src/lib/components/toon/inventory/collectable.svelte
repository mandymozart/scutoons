<script lang="ts">
	import { asset } from "$lib/utils/cdn.ts";

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
	}
	 img {
		width: 100%;
		transition: transform 0.2s ease;
		&:hover {
			transform: scale(1.1);
		}
	}
</style>