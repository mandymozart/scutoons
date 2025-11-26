<script lang="ts">
	import Swiper from 'swiper';
	import { asset } from '$lib/utils/cdn.ts';
	let { product, label } = $props();

	new Swiper('.swiper', {
		slidesPerView: 1,
		spaceBetween: 16,
		loop: true,
		autoplay: {
			delay: 5000,
			disableOnInteraction: true
		},
		navigation: {
			nextEl: `.swiper-button-next`,
			prevEl: `.swiper-button-prev`
		},
		breakpoints: {
			768: {
				slidesPerView: 3
			}
		}
	});
</script>

<div class="swiper gallery-tshirt-white">
	<div class="swiper-wrapper">
		{#each product.images as image}
			<div class="swiper-slide"><img src={asset(image)} alt={label} /></div>
		{/each}
	</div>
	<div class="swiper-button-prev"></div>
	<div class="swiper-button-next"></div>
</div>

<style>
	:root {
		--swiper-navigation-size: 0.75rem;
		--swiper-theme-color: black;
		--swiper-navigation-sides-offset: 0rem;
	}

	.swiper {
		margin-bottom: 1rem;
	}

	.swiper-slide {
		cursor: grab;
	}

	.swiper-slide img {
		width: 100%;
		height: auto;
		display: block;
	}
</style>
