<script lang="ts">
	import { onMount } from 'svelte';
	import Swiper from 'swiper';
	import { Autoplay, Navigation } from 'swiper/modules';
	import 'swiper/css';
	import 'swiper/css/navigation';
	import { asset } from '$lib/utils/cdn.ts';

	let { images, label }: { images: string[]; label: string } = $props();
	let swiperContainer: HTMLElement;
	let prevEl: HTMLElement;
	let nextEl: HTMLElement;

	onMount(() => {
		const swiper = new Swiper(swiperContainer, {
			modules: [Navigation, Autoplay],
			slidesPerView: 1,
			spaceBetween: 16,
			loop: true,
			autoplay: {
				delay: 5000,
				disableOnInteraction: true
			},
			navigation: {
				// scoped to this instance (several sliders on the homepage)
				nextEl,
				prevEl
			},
			breakpoints: {
				768: {
					slidesPerView: 3
				}
			}
		});
		return () => swiper.destroy();
	});
</script>

<div class="swiper" bind:this={swiperContainer}>
	<div class="swiper-wrapper">
		{#each images as image (image)}
			<div class="swiper-slide"><img src={asset(image)} alt={label} loading="lazy" /></div>
		{/each}
	</div>
	<div class="swiper-button-prev" bind:this={prevEl}></div>
	<div class="swiper-button-next" bind:this={nextEl}></div>
</div>

<style>
	:global(.swiper) {
		--swiper-navigation-size: 0.75rem;
		--swiper-theme-color: black;
		--swiper-navigation-sides-offset: 0rem;
	}

	.swiper {
		margin-bottom: 1rem;
		background: white;
	}

	:global(.swiper-slide) {
		cursor: grab;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	:global(.swiper-slide img) {
		width: 100%;
		height: auto;
		display: block;
		object-fit: contain;
	}
</style>
