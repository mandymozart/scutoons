<script lang="ts">
	import { page } from '$app/state';
</script>

<div class="view">
	<h1>{page.status}</h1>
	<p>{page.error?.message}</p>
</div>

<style>
	.view {
		max-width: 800px;
		margin: 5rem auto 0;
		padding: 0 1rem;
	}
</style>
