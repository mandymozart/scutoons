<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	// the toons are on Home now (its Toons list opens a toon's board)
	onMount(() => void goto('/', { replaceState: true }));
</script>
