<script lang="ts">
	import { onMount } from 'svelte';
	import Swiper from 'swiper';
	import { Navigation } from 'swiper/modules';
	import 'swiper/css';
	import 'swiper/css/navigation';
	import { asset } from '$lib/utils/cdn.ts';

	let { product, label } = $props();
	let swiperContainer;

	onMount(() => {
		new Swiper(swiperContainer, {
			modules: [Navigation],
			slidesPerView: 1,
			spaceBetween: 16,
			loop: true,
			autoplay: {
				delay: 5000,
				disableOnInteraction: true
			},
			navigation: {
				nextEl: '.swiper-button-next',
				prevEl: '.swiper-button-prev'
			},
			breakpoints: {
				768: {
					slidesPerView: 3
				}
			}
		});
	});
</script>

<div class="swiper gallery-tshirt-white" bind:this={swiperContainer}>
	<div class="swiper-wrapper">
		{#each product.images as image}
			<div class="swiper-slide"><img src={asset(image)} alt={label} /></div>
		{/each}
	</div>
	<div class="swiper-button-prev"></div>
	<div class="swiper-button-next"></div>
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
