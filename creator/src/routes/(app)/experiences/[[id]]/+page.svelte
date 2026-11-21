<script lang="ts">
	import { page } from '$app/state';
	import ExperienceWorkspace from '$lib/components/experiences/experience-workspace.svelte';

	const id = $derived(page.params.id ?? null);
</script>

<ExperienceWorkspace {id} />
