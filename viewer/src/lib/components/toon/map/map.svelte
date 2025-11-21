<script lang="ts">
	import { goto } from '$app/navigation';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { onClickOutside } from 'runed';
	import { getNavigationContext } from '$lib/contexts/navigation.svelte';

	interface Coordinates {
		x: number;
		y: number;
	}

	interface Location {
		id: string;
		label: string;
		toonId: string;
		chapterNumber?: string;
		songNumber?: string;
		type: string;
		image: string;
		coordinates: Coordinates;
	}

	interface Route {
		id: string;
		locations: string[];
	}

	interface MapData {
		locations: Location[];
		routes: Route[];
	}

	const MAP_WIDTH = 2048;
	const MAP_HEIGHT = 1046;
	const CENTER_X = 1320;
	const CENTER_Y = 440;

	const DATA: MapData = {
		locations: [
			{
				id: 'cathedral',
				toonId: 'catpope',
				label: 'Cathedral',
				type: 'chapter',
				chapterNumber: '1',
				image: '',
				coordinates: {
					x: 1182,
					y: 323
				}
			},
			{
				id: 'apartment',
				label: 'Apartment',
				toonId: 'catpope',
				type: 'chapter',
				chapterNumber: '1',
				image: '',
				coordinates: {
					x: 1206,
					y: 304
				}
			},
			{
				id: 'airport',
				label: 'Airport',
				toonId: 'catpope',
				type: 'chapter',
				chapterNumber: '1',
				image: '',
				coordinates: {
					x: 1254,
					y: 252
				}
			},
			{
				id: 'medival-street',
				label: 'Medival Street',
				toonId: 'catpope',
				type: 'chapter',
				chapterNumber: '1',
				image: '',
				coordinates: {
					x: 1250,
					y: 310
				}
			},
			{
				id: 'station',
				label: 'Station',
				toonId: 'catpope',
				type: 'chapter',
				chapterNumber: '1',
				image: '',
				coordinates: {
					x: 1202,
					y: 332
				}
			},{
				id: 'mountain-path',
				label: 'Mountain Path',
				toonId: 'catpope',
				type: 'chapter',
				chapterNumber: '1',
				image: '',
				coordinates: {
					x: 1448,
					y: 554
				}
			},
						{
				id: 'monastery',
				label: 'Monastery',
				toonId: 'catpope',
				type: 'chapter',
				chapterNumber: '1',
				image: '',
				coordinates: {
					x: 1428,
					y: 600
				}
			},
			{
				id: 'library',
				label: 'Library',
				toonId: 'catpope',
				type: 'chapter',
				chapterNumber: '1',
				image: '',
				coordinates: {
					x: 1431,
					y: 586
				}
			},{
				id: 'graveyard',
				label: 'Graveyard',
				toonId: 'catpope',
				type: 'chapter',
				chapterNumber: '1',
				image: '',
				coordinates: {
					x: 1441,
					y: 569
				}
			}
		],
		routes: [
			{
				id: 'season1',
				locations: ['cathedral', 'apartment', 'graveyard', 'airport', 'medival-street', 'monastery']
			}
		]
	};

	let {
		zoom = 1,
		coordinates = { x: 0, y: 0 }
	}: {
		zoom?: number;
		coordinates?: { x: number; y: number };
	} = $props();

	const nav = getNavigationContext();
	const mapVisible = $derived(nav.isOpen('map'));

	let mapContainer = $state<HTMLDivElement | null>(null);
	let mapElement = $state<HTMLDivElement>();
	let centerOffsetX = $state(0);
	let centerOffsetY = $state(0);

	// Calculate offset to center the world at (CENTER_X, CENTER_Y)
	$effect(() => {
		if (mapContainer) {
			centerOffsetX = mapContainer.clientWidth / 2 - CENTER_X;
			centerOffsetY = mapContainer.clientHeight / 2 - CENTER_Y;
		}
	});

	onClickOutside(
		() => mapElement,
		() => {
			if (mapVisible) nav.closeOverlay();
		}
	);

	// Keyboard shortcuts for map
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

	function handleLocationClick(location: Location) {
		console.log(`Location clicked: ${location.id}`);
		nav.closeOverlay();
		goto(localizeHref(`/${location.toonId}/${location.type}/${location.chapterNumber}?label=${location.id}`));
	}

	function getRouteLocations(route: Route): Location[] {
		return route.locations
			.map((locId) => DATA.locations.find((loc) => loc.id === locId))
			.filter((loc): loc is Location => loc !== undefined);
	}

	function createBezierPath(locations: Location[]): string {
		if (locations.length < 2) return '';

		const first = locations[0].coordinates;
		let path = `M ${first.x} ${first.y}`;

		// Handle length factor (0-1, lower = tighter curves)
		const tension = 0.3;

		for (let i = 1; i < locations.length; i++) {
			const curr = locations[i].coordinates;
			const prev = locations[i - 1].coordinates;

			// Calculate tangent direction at previous point
			let prevTangentX, prevTangentY;
			if (i === 1) {
				// First segment: tangent points to current
				prevTangentX = curr.x - prev.x;
				prevTangentY = curr.y - prev.y;
			} else {
				// Middle segments: tangent from prev-prev to current
				const prevPrev = locations[i - 2].coordinates;
				prevTangentX = curr.x - prevPrev.x;
				prevTangentY = curr.y - prevPrev.y;
			}

			// Calculate tangent direction at current point
			let currTangentX, currTangentY;
			if (i === locations.length - 1) {
				// Last segment: tangent from previous
				currTangentX = curr.x - prev.x;
				currTangentY = curr.y - prev.y;
			} else {
				// Middle segments: tangent from previous to next
				const next = locations[i + 1].coordinates;
				currTangentX = next.x - prev.x;
				currTangentY = next.y - prev.y;
			}

			// Normalize and scale tangents
			const prevLen = Math.sqrt(prevTangentX ** 2 + prevTangentY ** 2);
			const currLen = Math.sqrt(currTangentX ** 2 + currTangentY ** 2);
			
			const segmentLen = Math.sqrt((curr.x - prev.x) ** 2 + (curr.y - prev.y) ** 2);

			// Control points along tangent lines
			const cx1 = prev.x + (prevTangentX / prevLen) * segmentLen * tension;
			const cy1 = prev.y + (prevTangentY / prevLen) * segmentLen * tension;
			const cx2 = curr.x - (currTangentX / currLen) * segmentLen * tension;
			const cy2 = curr.y - (currTangentY / currLen) * segmentLen * tension;

			path += ` C ${cx1} ${cy1}, ${cx2} ${cy2}, ${curr.x} ${curr.y}`;
		}

		return path;
	}
</script>

<button class="map-toggle" onclick={toggleMap}><img src={'/icons/map.svg'} alt="Map" /></button>

<div class="map {mapVisible ? 'visible' : 'hidden'}" bind:this={mapElement}>
	<div class="map-viewport" bind:this={mapContainer}>
		<div
			class="map-content"
			style:transform="translate({centerOffsetX + coordinates.x}px, {centerOffsetY +
				coordinates.y}px) scale({zoom})"
			style:width="{MAP_WIDTH}px"
			style:height="{MAP_HEIGHT}px"
		>
			<!-- Background layer -->
			<div class="layer layer-background"></div>

			<!-- Terrain layer -->
			<div class="layer layer-terrain">
				<img
					src={asset('catpope/images/map/map-layer-terrain.svg')}
					alt="Map Terrain"
					width={MAP_WIDTH}
					height={MAP_HEIGHT}
				/>
			</div>

			<!-- Routes layer -->
			<div class="layer layer-routes">
				<svg width={MAP_WIDTH} height={MAP_HEIGHT} style="position: absolute; pointer-events: none;">
					{#each DATA.routes as route}
						{@const locations = getRouteLocations(route)}
						<path
							d={createBezierPath(locations)}
							stroke="black"
							stroke-width="8"
							fill="none"
							stroke-linecap="round"
							stroke-linejoin="round"
						/>
					{/each}
				</svg>
			</div>

			<!-- Locations layer -->
			<div class="layer layer-locations">
				{#each DATA.locations as location}
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
				{#each DATA.locations as location}
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
		/* border: 1rem solid black; */
		overflow: hidden;
		inset: 0;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 1000;
		background: white;
		transform: translate(0, 100%);
		transition: transform 0.1s ease;
	}
	.map.visible {
		transform: translate(0, 0);
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

	.layer-terrain img {
		display: block;
		width: 100%;
		height: 100%;
	}


	.location {
		position: absolute;
		transform: translate(-50%, -50%);
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
		position: absolute;
		transform: translateX(-50%);
		white-space: nowrap;
		font-size: .75rem;
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
</style>
