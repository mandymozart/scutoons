<script lang="ts">
	import { MediaSlot } from '@scutoons/media';
	import { problemFor, type CatalogFormProps } from '../kinds.ts';
	import { m } from '../messages.ts';
	import LocaleText from './locale-text.svelte';

	/**
	 * A carrier of the creature airlift (agents/research/CAVE.md D8): a crazy creature or a flying robot taxi – its name and
	 * its picture (a cut-out). One is drawn at random per flight; uploads go to scutoons/airlift/.
	 */
	let { data = $bindable(), problems }: CatalogFormProps = $props();

	const err = (...words: string[]) => problemFor(problems, ...words);
	let bust = $state(0);
</script>

<LocaleText label={m['catalog.name']()} required value={data.name} error={err('name')} onchange={(v) => (data.name = v)} testid="airlift-carrier-name" />
<MediaSlot toon="" value={data.image || undefined} label={m['catalog.picture']()} place="scutoons/airlift/" {bust} onchange={(path) => ((data.image = path ?? ''), (bust = Date.now()))} testid="airlift-carrier-image" />
{#if err('image')}<p class="sc-error">{err('image')}</p>{/if}
