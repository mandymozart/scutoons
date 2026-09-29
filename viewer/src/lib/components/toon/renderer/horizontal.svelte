<script lang="ts">
	import type { Group } from '@scutoons/content';
	import Slice from '../slice.svelte';
	import Swiper from 'swiper';
	import { Autoplay, EffectCube, FreeMode, Navigation, Keyboard, EffectFlip, Mousewheel } from 'swiper/modules';
	import 'swiper/css';
	import 'swiper/css/effect-cube';
	import 'swiper/css/effect-flip';
	import 'swiper/css/navigation';
	import swipeIcon from '$lib/icons/horizontal-swipe-gesture.svg';

	let { group }: { group: Group } = $props();
	let swiperContainer: HTMLElement;
	let swiperInstance: Swiper | null = null;
	let showNavigationHint = $state(false);
	let hasShownHint = false;
	let fadeOutTimeout: ReturnType<typeof setTimeout> | null = null;

	$effect(() => {
		if (swiperContainer) {
			const params = (typeof group.renderer === 'object' ? group.renderer.params : undefined) || {};
			const modules = [Navigation, Keyboard, Mousewheel];

			if (params.autoPlay?.enabled) modules.push(Autoplay);
			if (params.cube) modules.push(EffectCube);
			if (params.flip) modules.push(EffectFlip);
			if (params.freeMode) modules.push(FreeMode);

			const hideHint = () => {
				if (fadeOutTimeout) {
					clearTimeout(fadeOutTimeout);
					fadeOutTimeout = null;
				}
				showNavigationHint = false;
			};

			swiperInstance = new Swiper(swiperContainer, {
				modules,
				slidesPerView: 1,
				spaceBetween: 0,
				loop: params.loop || false,
				mousewheel: {
					enabled: true,
					forceToAxis: false,
					sensitivity: 1,
					releaseOnEdges: false
				},
				...(params.keyboard && {
					keyboard: {
						enabled: true,
						onlyInViewport: true
					}
				}),

				...(params.cube && {
					effect: 'cube',
					cubeEffect: {
						shadow: true,
						slideShadows: true,
						shadowOffset: 20,
						shadowScale: 0.94
					}
				}),

				...(params.flip && {
					effect: 'flip',
					flipEffect: {
						slideShadows: true,
						limitRotation: true
					}
				}),

				...(params.autoPlay?.enabled && {
					autoplay: {
						delay: params.autoPlay.delay || 5000,
						disableOnInteraction: true
					}
				}),

				...(params.freeMode && {
					freeMode: {
						enabled: true,
						momentum: true,
						momentumBounce: true,
						momentumRatio: 1,
						momentumVelocityRatio: 1,
						sticky: false
					}
				}),

				navigation: {
					nextEl: '.swiper-button-next',
					prevEl: '.swiper-button-prev'
				},
				on: {
					slideChange: hideHint,
					touchStart: hideHint,
					keyPress: hideHint
				}
			});
		}

		return () => {
			if (swiperInstance) {
				swiperInstance.destroy();
				swiperInstance = null;
			}
		};
	});

	$effect(() => {
		if (!swiperContainer) return;

		let hintTimeout: ReturnType<typeof setTimeout> | null = null;

		const observer = new IntersectionObserver(
			([entry]) => {
				if (entry.isIntersecting && !hasShownHint) {
					hasShownHint = true;

					hintTimeout = setTimeout(() => {
						showNavigationHint = true;
						fadeOutTimeout = setTimeout(() => {
							showNavigationHint = false;
							fadeOutTimeout = null;
						}, 1500);
					}, 1000);
				} else if (!entry.isIntersecting) {
					// Reset when scrolling away so hint can show again on return
					hasShownHint = false;
					if (hintTimeout) {
						clearTimeout(hintTimeout);
						hintTimeout = null;
					}
				}
			},
			{ threshold: 0.5 }
		);

		observer.observe(swiperContainer);

		return () => {
			observer.disconnect();
			if (fadeOutTimeout) {
				clearTimeout(fadeOutTimeout);
			}
			if (hintTimeout) {
				clearTimeout(hintTimeout);
			}
		};
	});
</script>

<div
	class="renderer renderer--horizontal swiper"
	data-group-label={group.label}
	bind:this={swiperContainer}
>
	<div class="swiper-wrapper">
		{#each group.slices as slice, index}
			<div class="swiper-slide">
				<Slice {slice} sliceIndex={index} allSlices={group.slices} />
			</div>
		{/each}
	</div>
	<div class="swiper-button-next" class:show-hint={showNavigationHint}>
		<img src={swipeIcon} alt="Swipe" />
	</div>
</div>

<style>
	/* .renderer--horizontal.swiper {
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.renderer--horizontal.swiper	.swiper-wrapper {
						display: flex;
			align-items: center;
			justify-content: center;
			height: 100vh;
		
	} */
	.swiper-button-next {
		opacity: 0;
		transition: opacity 0.3s ease-in-out;
		pointer-events: none;
		color: black;
		width: 5rem;
    
	}
	.swiper-slide {
		max-height: 100dvh;
	}

	.swiper-button-next.show-hint {
		opacity: 1;
		pointer-events: auto;
	}

	.swiper-button-next.show-hint img {
		animation: swipe-gesture 1s ease-in-out infinite;
	}

	@keyframes swipe-gesture {
		0% {
			transform: translateX(0) rotate(0deg);
			animation-timing-function: ease-out;
		}
		30% {
			transform: translateX(0.6rem) rotate(12deg);
			animation-timing-function: ease-in-out;
		}
		45% {
			transform: translateX(0) rotate(0deg);
			animation-timing-function: ease-in;
		}
		100% {
			transform: translateX(0) rotate(0deg);
		}
	}
</style>
