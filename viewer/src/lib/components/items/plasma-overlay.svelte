<script lang="ts">
	import { onMount } from 'svelte';

	/**
	 * Farbrausch's plasma cloud (owner 2026-10-08: "a plasma cloud which we can overlay with some blending effect, that is
	 * moving"): the classic sine plasma, computed small (about 96 pixels across) and scaled up softly with a blur so it
	 * becomes a cloud, fixed over the whole screen (text and controls too), blended with --sc-farbrausch-blend at
	 * --sc-farbrausch-opacity; --sc-farbrausch-speed scales its movement. Never takes a tap. About 30 frames a second,
	 * none while the tab is hidden.
	 */
	let canvas: HTMLCanvasElement;

	onMount(() => {
		const g = canvas.getContext('2d');
		if (!g) return;
		const W = 96;
		let H = 0;
		let image: ImageData | null = null;
		function size() {
			H = Math.max(24, Math.round((W * window.innerHeight) / Math.max(1, window.innerWidth)));
			canvas.width = W;
			canvas.height = H;
			image = g!.createImageData(W, H);
		}
		size();
		window.addEventListener('resize', size);

		let frame = 0;
		let last = 0;
		let t = Math.random() * 100;
		const speed = () => {
			const v = parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--sc-farbrausch-speed'));
			return Number.isFinite(v) && v >= 0 ? v : 1;
		};
		let k = speed();
		let tick = 0;

		function draw(now: number) {
			frame = requestAnimationFrame(draw);
			if (now - last < 33 || !image) return;
			const dt = last ? Math.min(0.1, (now - last) / 1000) : 0;
			last = now;
			if (++tick % 30 === 0) k = speed();
			t += dt * 0.6 * k;
			const d = image.data;
			const cx = W / 2 + Math.sin(t * 0.7) * W * 0.3;
			const cy = H / 2 + Math.cos(t * 0.5) * H * 0.3;
			for (let y = 0; y < H; y++) {
				for (let x = 0; x < W; x++) {
					const v =
						Math.sin(x * 0.09 + t) +
						Math.sin(y * 0.07 - t * 1.3) +
						Math.sin((x + y) * 0.05 + t * 0.7) +
						Math.sin(Math.hypot(x - cx, y - cy) * 0.12 - t * 1.1);
					const p = v * 0.8 + t * 0.4;
					const i = (y * W + x) * 4;
					d[i] = 128 + 127 * Math.sin(p);
					d[i + 1] = 128 + 127 * Math.sin(p + 2.094);
					d[i + 2] = 128 + 127 * Math.sin(p + 4.189);
					d[i + 3] = 255;
				}
			}
			g!.putImageData(image, 0, 0);
		}
		frame = requestAnimationFrame(draw);

		return () => {
			cancelAnimationFrame(frame);
			window.removeEventListener('resize', size);
		};
	});
</script>

<canvas class="plasma" bind:this={canvas} aria-hidden="true" data-testid="farbrausch-plasma"></canvas>

<style>
	:global(:root) {
		--sc-farbrausch-blend: overlay;
		--sc-farbrausch-opacity: 0.55;
		--sc-farbrausch-speed: 1;
	}
	.plasma {
		position: fixed;
		/* a little past the edges: the blur's soft border stays outside the screen */
		inset: -2rem;
		width: calc(100% + 4rem);
		height: calc(100% + 4rem);
		z-index: 990;
		pointer-events: none;
		mix-blend-mode: var(--sc-farbrausch-blend);
		opacity: var(--sc-farbrausch-opacity);
		filter: blur(1.5rem) saturate(1.3);
	}
</style>
