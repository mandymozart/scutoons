<script lang="ts">
	/**
	 * The turntable picture (owner 2026-10-14 / 10-16): a record with the album's cover as its label, a tonearm from the
	 * upper right. It turns while a song plays; the arm sits on the outer groove at a song's start, moves in as it plays
	 * (`armAngle`) and lifts between songs. "Turn the record" (`side` b) flips it to its back – a paper label with the
	 * album's title. Drawn at any size: every inner size is in em of the deck (`size` / 3.25), so the compact bar (3.25rem)
	 * and the expanded card draw the same picture. Reduced motion: no spinning, the arm jumps, the flip has no animation.
	 *
	 * The card shows the item's 3D model instead when it has one (record-stage.svelte, FEATURES 10.102); this 2D picture
	 * stays the bar's and the card's fallback (reduced motion, no WebGL, a model that fails).
	 */
	let {
		cover = null,
		title = '',
		spinning = false,
		arm,
		lifted = false,
		side = 'a',
		size = '3.25rem'
	}: {
		/** the label picture (the album cover) */
		cover?: string | null;
		/** the back label's text (the album title) */
		title?: string;
		spinning?: boolean;
		/** the tonearm's angle in degrees (`armAngle`) */
		arm: number;
		lifted?: boolean;
		side?: 'a' | 'b';
		size?: string;
	} = $props();
</script>

<div class="deck" class:spinning class:turned={side === 'b'} style:--deck={size} style:--arm="{arm}deg" aria-hidden="true">
	<div class="flipper">
		<div class="face front">
			<div class="disc">
				<span class="label">{#if cover}<img src={cover} alt="" />{/if}</span>
			</div>
		</div>
		<div class="face back">
			<div class="disc">
				<span class="label paper"><span class="name">{title}</span><span class="letter">B</span></span>
			</div>
		</div>
	</div>
	<div class="arm" class:lifted></div>
</div>

<style>
	.deck {
		position: relative;
		flex: none;
		width: var(--deck);
		height: var(--deck);
		font-size: calc(var(--deck) / 3.25);
		perspective: 40em;
	}

	.flipper {
		position: absolute;
		inset: 0.125em;
		transform-style: preserve-3d;
		transition: transform 0.3s ease;
	}

	.turned .flipper {
		transform: rotateY(180deg);
	}

	.face {
		position: absolute;
		inset: 0;
		backface-visibility: hidden;
		-webkit-backface-visibility: hidden;
	}

	.back {
		transform: rotateY(180deg);
	}

	/* the vinyl: record art, not UI chrome – near-black grooves */
	.disc {
		width: 100%;
		height: 100%;
		border-radius: 50%;
		background: repeating-radial-gradient(circle, #1a1a1a 0 0.09em, #2b2b2b 0.09em 0.14em);
		display: grid;
		place-items: center;
		animation: spin 1.8s linear infinite;
		animation-play-state: paused;
	}

	.spinning .disc {
		animation-play-state: running;
	}

	.label {
		width: 42%;
		height: 42%;
		border-radius: 50%;
		overflow: hidden;
		display: grid;
		place-items: center;
		background: var(--ink);
	}

	.label img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.label.paper {
		background: var(--paper);
		color: var(--ink);
		align-content: center;
		gap: 0.06em;
		text-align: center;
	}

	.name {
		font-size: 0.16em;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		max-width: 80%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.letter {
		font-size: 0.28em;
		line-height: 1;
	}

	.arm {
		position: absolute;
		top: -0.125em;
		right: -0.125em;
		width: 0.25em;
		height: 2.1em;
		background: linear-gradient(#c9c9c9, #8a8a8a);
		border: 0.0625em solid var(--ink);
		border-radius: 0.125em;
		transform-origin: 50% 0.125em;
		transform: rotate(var(--arm)) translateY(0);
		transition: transform 0.6s ease;
	}

	.arm.lifted {
		transform: rotate(var(--arm)) translateY(-0.2em) scale(1.06);
	}

	@keyframes spin {
		to {
			transform: rotate(1turn);
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.disc {
			animation: none;
		}
		.arm,
		.flipper {
			transition: none;
		}
	}
</style>
