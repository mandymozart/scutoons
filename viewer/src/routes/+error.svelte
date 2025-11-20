<script lang="ts">
	import {page } from "$app/state"
	$inspect(page.error)
</script>

<div class="view">

	<h1>Upsi! Mea culpa!</h1>
	<p>{page.error?.message}</p>
</div>

<style>
	.view {
	margin-top: 4rem;
	}
</style>
