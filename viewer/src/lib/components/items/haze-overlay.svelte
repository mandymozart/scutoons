<script lang="ts">
	/**
	 * The `haze` reader effect ("Dunst", owner 2026-10-08: weed – "not a party drug … we display something overlaying the
	 * screen, in haze"): a light veil that softens everything a little, and big soft banks of haze drifting slowly across,
	 * fixed over the whole screen, never taking a tap. Tuning in the inspector: --sc-haze-color, --sc-haze-opacity,
	 * --sc-haze-blur, --sc-haze-blend.
	 */
	const banks = [
		{ x: 10, y: 15, size: 90, ms: 47000, delay: -9000 },
		{ x: 60, y: 45, size: 110, ms: 61000, delay: -27000 },
		{ x: 20, y: 75, size: 100, ms: 53000, delay: -14000 },
		{ x: 75, y: 5, size: 80, ms: 69000, delay: -40000 }
	];
</script>

<div class="haze" aria-hidden="true" data-testid="haze-overlay">
	{#each banks as b, i (i)}
		<span class="bank" style:left="{b.x - b.size / 2}%" style:top="{b.y - b.size / 2}%" style:width="{b.size}vmax" style:height="{b.size * 0.6}vmax" style:animation-duration="{b.ms}ms" style:animation-delay="{b.delay}ms"></span>
	{/each}
</div>

<style>
	:global(:root) {
		--sc-haze-color: 236, 232, 222;
		--sc-haze-opacity: 0.85;
		--sc-haze-blur: 0.0625rem;
		--sc-haze-blend: screen;
	}
	.haze {
		position: fixed;
		inset: 0;
		z-index: 990;
		overflow: hidden;
		pointer-events: none;
		opacity: var(--sc-haze-opacity);
		background: rgba(var(--sc-haze-color), 0.14);
		backdrop-filter: blur(var(--sc-haze-blur)) saturate(0.85);
		-webkit-backdrop-filter: blur(var(--sc-haze-blur)) saturate(0.85);
		mix-blend-mode: var(--sc-haze-blend);
		animation: haze-in 3s ease-out both;
	}
	.bank {
		position: absolute;
		border-radius: 50%;
		background: radial-gradient(ellipse at center, rgba(var(--sc-haze-color), 0.55) 0%, rgba(var(--sc-haze-color), 0.25) 40%, rgba(var(--sc-haze-color), 0) 70%);
		filter: blur(1.5rem);
		animation-name: haze-drift;
		animation-timing-function: ease-in-out;
		animation-iteration-count: infinite;
		animation-direction: alternate;
	}
	@keyframes haze-in {
		from {
			opacity: 0;
		}
	}
	@keyframes haze-drift {
		0% {
			transform: translate(-12vw, 0) scale(1);
		}
		50% {
			transform: translate(6vw, -6vh) scale(1.15);
		}
		100% {
			transform: translate(14vw, 4vh) scale(0.95);
		}
	}
</style>
