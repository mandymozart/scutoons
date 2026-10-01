<script lang="ts">
	/**
	 * A toon's navigation avatar (`<toonId>/avatar.webp` on the CDN, square, transparent): `radius-sm`, no border,
	 * `object-fit: contain`, lazy. Without `src` or when it fails to load: a neutral placeholder with the first letter
	 * of `name` (default `alt`) on `surface-hover`.
	 */
	let {
		src,
		alt = '',
		name,
		size = '2.5rem'
	}: {
		src?: string | null;
		alt?: string;
		/** placeholder letter source; defaults to `alt` */
		name?: string;
		/** CSS length for width and height */
		size?: string;
	} = $props();

	let failedSrc = $state<string | null>(null);
	const showImage = $derived(!!src && failedSrc !== src);
	const letter = $derived((name ?? alt).trim().charAt(0).toUpperCase());
</script>

{#if showImage}
	<img class="sc-toon-avatar" style:--toon-avatar-size={size} {src} {alt} loading="lazy" decoding="async" onerror={() => (failedSrc = src ?? null)} />
{:else}
	<span
		class="sc-toon-avatar placeholder"
		style:--toon-avatar-size={size}
		role={alt ? 'img' : undefined}
		aria-label={alt || undefined}
		aria-hidden={alt ? undefined : 'true'}>{letter}</span
	>
{/if}

<style>
	.sc-toon-avatar {
		display: block;
		flex: none;
		box-sizing: border-box;
		width: var(--toon-avatar-size);
		height: var(--toon-avatar-size);
		margin: 0;
		border: 0;
		border-radius: var(--radius-sm);
		object-fit: contain;
	}
	.placeholder {
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--surface-hover);
		color: var(--text-muted);
		font-weight: var(--font-weight-bold);
		font-size: max(var(--type-caption-size), calc(var(--toon-avatar-size) * 0.45));
		line-height: 1;
		user-select: none;
	}
</style>
