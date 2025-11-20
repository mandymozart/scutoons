<script lang="ts">
	import SequenceNavigation from '$lib/components/toon/sequence-navigation.svelte';
	import { goto } from '$app/navigation';
	import { setContext, onDestroy } from 'svelte';
	import { setAudioContext } from '$lib/contexts/audio.svelte.ts';

	let { children, data, params } = $props();
	let sequence = data.sequence || [];
	$inspect(data);

	function navigate(url: string | URL, opts?: Parameters<typeof goto>[1]): void {
		goto(url, opts);
	}
	setContext('navigation', navigate);

	// Initialize audio context for entire toon lifetime
	const audio = setAudioContext(params.toonId);

	// Cleanup when leaving toon entirely
	onDestroy(async () => {
		console.log('🎵 Toon layout destroyed, cleaning up audio');
		await audio.unload();
	});
	$inspect(audio);
</script>

{@render children()}

<SequenceNavigation {sequence} {params} /> 
