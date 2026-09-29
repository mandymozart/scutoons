<script lang="ts">
	import { getLocale, locales, setLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { WWW_URL } from '$lib/config.ts';
	import { Button } from '@scutoons/ui';

	const locale = getLocale();
</script>

<!-- Same place and size as the viewer's logo (main menu toggle); here it leads back to www -->
<header class="shop-nav">
	<a class="home" href={`${WWW_URL}/`} aria-label={m['nav.home']()} title={m['nav.home']()}>
		<img src="/SCUTOONS-logo-animated.svg" alt="SCUTOONS" />
	</a>
	<nav class="language" aria-label={m['nav.language']()}>
		{#each locales as l (l)}
			<Button
				variant={l === locale ? 'primary' : 'secondary'}
				size="sm"
				aria-pressed={l === locale}
				onclick={() => setLocale(l)}>{l}</Button
			>
		{/each}
	</nav>
</header>

<style>
	.home {
		position: fixed;
		top: 0.5rem;
		left: 0;
		z-index: 10;
		display: block;
		touch-action: manipulation;
	}

	.home img {
		height: 3rem;
	}

	.language {
		position: fixed;
		top: 0.5rem;
		right: 0.5rem;
		z-index: 10;
		display: flex;
		gap: var(--space-025);
	}
</style>
