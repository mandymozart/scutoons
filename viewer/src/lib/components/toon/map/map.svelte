<script lang="ts">
	import { goto } from '$app/navigation';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { asset } from '$lib/utils/cdn.ts';
	import type { MapLocation, MapCoordinates, MapRoute, Map, Toon } from '$lib/models/content.ts';
	import { onClickOutside } from 'runed';
	import { getNavigationContext } from '$lib/contexts/navigation.svelte';
	import { page } from '$app/state';
	import Routes from './routes.svelte';
	import Layers from './layers.svelte';
	import mapIcon from '$lib/icons/map.svg';

	let {
		zoom = 1,
		coordinates = { x: 0, y: 0 },
		locations = [],
		routes = []
	}: {
		zoom?: number;
		coordinates?: MapCoordinates;
		locations: MapLocation[];
		routes: MapRoute[];
	} = $props();

	const nav = getNavigationContext();
	const mapVisible = $derived(nav.isOpen('map'));

	const toon = $derived(page.data.toon as Toon);
	const map = $derived(toon.map as Map);

	let mapContainer = $state<HTMLDivElement | null>(null);
	let mapElement = $state<HTMLDivElement>();
	let centerOffsetX = $state(0);
	let centerOffsetY = $state(0);

	// Calculate offset to center the world at (map.center.x, map.center.y)
	$effect(() => {
		if (mapContainer) {
			centerOffsetX = mapContainer.clientWidth / 2 - map.center.x;
			centerOffsetY = mapContainer.clientHeight / 2 - map.center.y;
		}
	});

	onClickOutside(
		() => mapElement,
		() => {
			if (mapVisible) nav.closeOverlay();
		}
	);

	$effect(() => {
		function handleKeyDown(e: KeyboardEvent) {
			if (e.key === 'Escape' && mapVisible) {
				nav.closeOverlay();
			} else if (e.key === 'm' || e.key === 'M') {
				nav.toggleOverlay('map');
			}
		}

		window.addEventListener('keydown', handleKeyDown);
		return () => window.removeEventListener('keydown', handleKeyDown);
	});

	function toggleMap(e?: MouseEvent) {
		e?.stopPropagation();
		nav.toggleOverlay('map');
	}

	function handleLocationClick(location: MapLocation) {
		nav.closeOverlay();
		goto(
			localizeHref(
				`/${location.toonId}/${location.type}/${location.chapterNumber}?label=${location.id}`
			)
		);
	}
	$inspect(map);
</script>

<button class="map-toggle" onclick={toggleMap}><img src={mapIcon} alt="Map" /></button>

<div class="map {mapVisible ? 'visible' : 'hidden'}" bind:this={mapElement}>
	<img class="frame frame--1" src={asset(`catpope/images/vectors/vector-demon-saw-1.svg`)} alt="" />
	<img class="frame frame--2" src={asset(`catpope/images/vectors/vector-demon-saw-1.svg`)} alt="" />
	<img class="frame frame--3" src={asset(`catpope/images/vectors/vector-demon-saw-1.svg`)} alt="" />
	<img class="frame frame--4" src={asset(`catpope/images/vectors/vector-demon-saw-1.svg`)} alt="" />

	<div class="map-viewport" bind:this={mapContainer}>
		<div
			class="map-content"
			style:transform="translate({centerOffsetX + coordinates.x}px, {centerOffsetY +
				coordinates.y}px) scale({zoom})"
			style:width="{map.width}px"
			style:height="{map.height}px"
		>
			<!-- Background layer -->
			<div class="layer layer-background"></div>

			<!-- Custom layers -->
			<Layers layers={map.layers} width={map.width} height={map.height} />

			<!-- Routes layer -->
			<div class="layer layer-routes">
				<Routes width={map.width} height={map.height} {routes} {locations} />
			</div>

			<!-- Locations layer -->
			<div class="layer layer-locations">
				{#each locations as location}
					<button
						class="location"
						style:left="{location.coordinates.x}px"
						style:top="{location.coordinates.y}px"
						onclick={() => handleLocationClick(location)}
						title={location.label}
					>
					</button>
				{/each}
			</div>

			<!-- Labels layer -->
			<div class="layer layer-labels">
				{#each locations as location}
					<span
						class="label"
						style:left="{location.coordinates.x}px"
						style:top="{location.coordinates.y - 25}px"
					>
						{location.label}
					</span>
				{/each}
			</div>
		</div>
	</div>
</div>

<style>
	.map-toggle {
		position: fixed;
		top: 0.5rem;
		right: 3rem;
		z-index: 1001;
		background: white;
		border: 2px solid black;
		padding: 0;
		border-radius: 50%;
		height: 2rem;
		width: 2rem;
		display: flex;
		cursor: pointer;
		transition: all 0.2s ease;
		align-items: center;
		justify-content: center;
		touch-action: manipulation;
		img {
			width: 1rem;
		}
		&:hover {
			background: var(--primary-color);
		}
	}

	.map {
		position: fixed;
		inset: 0;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 1000;
		background: #00d1ff;
		opacity: 0;
		transform: translate(0, 100%) scale(1.1);
		transition:
			opacity 0.2s ease,
			transform 0.4s
				linear(
					0,
					0.03 1.1%,
					0.125 2.4%,
					0.906 9.8%,
					1.046 12.3%,
					1.11 15%,
					1.116 16.3%,
					1.11 17.8%,
					1.014 25.8%,
					0.987 31.2%,
					1.001 47.2%,
					1
				);
		border: 0 solid black;
		.frame {
			position: absolute;
			pointer-events: none;
			filter: drop-shadow(0px 0px 13px var(--primary-color));
			z-index: 1;
			transition: transform 2s
				linear(
					0,
					0.03 1.1%,
					0.125 2.4%,
					0.906 9.8%,
					1.046 12.3%,
					1.11 15%,
					1.116 16.3%,
					1.11 17.8%,
					1.014 25.8%,
					0.987 31.2%,
					1.001 47.2%,
					1
				);
			&.frame--1 {
				top: 0px;
				right: 0px;
				height: 105vw;
				transform: rotate(90deg);
				transform-origin: top;
				filter: drop-shadow(0px 0px 13px var(--color-acid-purple));
			}
			&.frame--2 {
				top: 0px;
				right: -37px;
				height: 100vh;
				filter: drop-shadow(-10px 50px 33px var(--color-acid-purple));
				transform: rotateY(180deg);
				/* transform-origin: top; */
			}
			&.frame--3 {
				bottom: 0px;
				right: 0;
				height: 105vw;
				transform: rotate(-90deg);
				transform-origin: bottom;
				filter: drop-shadow(0px 0px 13px var(--color-acid-purple));
			}
			&.frame--4 {
				top: 0px;
				left: -33px;
				height: 100vh;
				filter: drop-shadow(0px 0px 13px var(--color-acid-purple));
			}
		}
	}

	.map-viewport {
		width: 100%;
		height: 100%;
		overflow: hidden;
		position: relative;
	}

	.map-content {
		position: relative;
		transform-origin: 0 0;
	}

	.layer {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}

	.layer-background {
		background: var(--primary-color);
	}

	.location {
		opacity: 0;
		position: absolute;
		transform: translate(-50%, -50%);
		transition-delay: 5s;
		transition: opacity 2s
			linear(
				0,
				0.109 9.8%,
				0.259 15.8%,
				0.995 27.8%,
				0.396 38.4%,
				0.526 42.6%,
				0.996 48%,
				0.582 55%,
				0.999 62%,
				0.724 67.4%,
				0.999 73.4%,
				0.837 77.8%,
				1 83.2%,
				0.923 86.8%,
				1 92%,
				1
			);
		font-size: 20px;
		line-height: 1;
		padding: 0;
		width: 1rem;
		height: 1rem;
		background: #f0fcff;
		border: 2px solid black;
		border-radius: 50%;
		pointer-events: auto;
		&:hover {
			background: var(--primary-color);
		}
	}

	.label {
		opacity: 0;
		position: absolute;
		transform: translateX(-50%);
		transition-delay: 4s;
		transition: opacity 1s

			linear(
				0,
				0.109 9.8%,
				0.259 15.8%,
				0.995 27.8%,
				0.396 38.4%,
				0.526 42.6%,
				0.996 48%,
				0.582 55%,
				0.999 62%,
				0.724 67.4%,
				0.999 73.4%,
				0.837 77.8%,
				1 83.2%,
				0.923 86.8%,
				1 92%,
				1
			);

		white-space: nowrap;
		font-size: 0.75rem;
		pointer-events: none;
		text-shadow:
			-1px -1px 0 white,
			0 -1px 0 white,
			1px -1px 0 white,
			1px 0 0 white,
			1px 1px 0 white,
			0 1px 0 white,
			-1px 1px 0 white,
			-1px 0 0 white;
	}

	.map.visible {
		opacity: 1;
		transform: translate(0, 0) scale(1);
		.frame--1 {
			transform: rotate(90deg) translate(3px, -6px);
		}
		.frame--2 {
			transform: translate(-13px, -8px);
		}
		.frame--3 {
			transform: rotate(-90deg) translate(1px, 25px);
		}
		.frame--4 {
			transform: translate(13px, 8px);
		}
		.location {
			opacity: 1;
		}
		.label {
			opacity: 1;
		}
	}
</style>
