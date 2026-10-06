<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The source-language mark (owner 2026-10-06): the Feather `star` the toon page uses for the source, one size and
	 * placement everywhere – right after the language code it marks; tooltip "Source language".
	 */
</script>

<span class="source-mark" role="img" title={m['creator.source_language']()} aria-label={m['creator.source_language']()} data-testid="source-mark"><Icon name="star" size="0.9em" /></span>

<style>
	.source-mark {
		display: inline-flex;
		align-items: center;
		margin-left: var(--space-025);
		vertical-align: -0.1em;
		color: currentColor;
	}
</style>
