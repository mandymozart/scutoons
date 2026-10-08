<script lang="ts">
	import { CloseIcon } from '@scutoons/ui';
	import { setLocale, localizeHref, locales, getLocale } from '$lib/paraglide/runtime.js';
	import { onMount, tick } from 'svelte';
	import { holdChrome } from '$lib/chrome.ts';
	import { loadStockists } from '$lib/stockists.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { asset, toonImage } from '$lib/utils/cdn.ts';
	import { shopUrl } from '$lib/config.ts';
	import { toonName, type ToonIndex } from '@scutoons/content';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { getSessionContext } from '$lib/contexts/session.svelte.ts';
	import { onClickOutside } from 'runed';
	import accountIcon from '$lib/icons/account.svg';
	import menuIcon from '$lib/icons/menu.svg';

	let { toons }: { toons: ToonIndex[] } = $props();

	const membershipCtx = getMembershipContext();
	const session = getSessionContext();
	const isValid = $derived(membershipCtx.isValid);
	// the shop is its own app (shop.scutoons.com)
	const shopHref = $derived(shopUrl(getLocale()));

	// "Wo kaufen" only once a store is listed (agents/EMBASSY.md "Stores": no empty page in the menu before that)
	let hasStores = $state(false);
	onMount(() => {
		loadStockists().then((l) => (hasStores = !!l?.stores.length));
	});

	let menuOpen = $state(false);
	// the top row stays while the menu is open (lib/chrome.ts)
	$effect(() => {
		if (menuOpen) return holdChrome();
	});
	let menuElement = $state<HTMLDivElement>();
	let toggleElement = $state<HTMLButtonElement>();

	async function toggleMenu(e?: MouseEvent | KeyboardEvent) {
		e?.stopPropagation();
		menuOpen = !menuOpen;
		if (menuOpen) {
			// keyboard users land on the first toon
			await tick();
			(menuElement?.querySelector<HTMLElement>('.nav--toons a.nav-btn') ?? menuElement?.querySelector<HTMLElement>('.nav-btn'))?.focus();
		}
	}

	function closeMenu() {
		menuOpen = false;
	}

	// drag to close (owner 2026-10-01): a touch/pen swipe to the right follows the finger and closes past a threshold;
	// there is no swipe to open (it would clash with the horizontal slide reader in chapters)
	let drag = $state<{ x: number; y: number; dx: number; id: number; horizontal: boolean | null } | null>(null);
	const CLOSE_AT = 80;

	function dragStart(e: PointerEvent) {
		if (!menuOpen || e.pointerType === 'mouse') return;
		drag = { x: e.clientX, y: e.clientY, dx: 0, id: e.pointerId, horizontal: null };
	}
	function dragMove(e: PointerEvent) {
		if (!drag || e.pointerId !== drag.id) return;
		const dx = e.clientX - drag.x;
		const dy = e.clientY - drag.y;
		// decide once: a mostly horizontal gesture drags the menu, a vertical one scrolls it
		if (drag.horizontal === null && Math.abs(dx) + Math.abs(dy) > 8) drag.horizontal = Math.abs(dx) > Math.abs(dy);
		if (drag.horizontal) drag.dx = Math.max(0, dx);
	}
	function dragEnd(e: PointerEvent) {
		if (!drag || e.pointerId !== drag.id) return;
		dragMove(e);
		if (drag.horizontal && drag.dx > CLOSE_AT) closeMenu();
		drag = null;
	}

	onClickOutside(
		() => menuElement,
		() => {
			if (menuOpen) closeMenu();
		}
	);

	$effect(() => {
		function handleKeyDown(e: KeyboardEvent) {
			if (e.key === 'Escape' && menuOpen) {
				closeMenu();
				// back to the button that opened it
				toggleElement?.focus();
			}
		}

		window.addEventListener('keydown', handleKeyDown);
		return () => window.removeEventListener('keydown', handleKeyDown);
	});
</script>

<!-- the slide-out opens only from the menu button (top-right icon group, all widths) -->
<button
	bind:this={toggleElement}
	class="menu-toggle"
	onclick={toggleMenu}
	aria-label={m['menu.open']()}
	aria-expanded={menuOpen}
	aria-controls="main-menu"
	data-testid="menu-toggle"
>
	<img src={menuIcon} alt="" />
</button>

<!-- the logo is the link to the landing page (localized); on the landing page itself it just stays there -->
<a class="logo-link" href={localizeHref('/')} onclick={closeMenu} data-testid="logo-link">
	<img src="/SCUTOONS-logo-animated.svg" alt={m['menu.home']()} />
</a>

<div class="main-menu">
	<!-- closed: off screen and inert (not focusable, hidden from assistive tech) -->
	<div
		bind:this={menuElement}
		id="main-menu"
		class={`menu ${menuOpen ? 'open' : 'closed'}`}
		class:dragging={!!drag?.horizontal}
		style:transform={drag?.horizontal && drag.dx > 0 ? `translateX(${drag.dx}px)` : undefined}
		inert={!menuOpen}
		onpointerdown={dragStart}
		onpointermove={dragMove}
		onpointerup={dragEnd}
		onpointercancel={dragEnd}
	>
		<div class="menu-header">
			<nav class="nav nav--account" aria-label={m['account.my_account']()}>
				<ul class="nav-list">
					<li class="nav-item">
						<a class="nav-btn" href={localizeHref('/account')} onclick={closeMenu}>
							<img src={accountIcon} alt="" />
							<span>{session.isLoggedIn ? m['account.my_account']() : m['account.login_title']()}</span>
						</a>
					</li>
				</ul>
			</nav>
			<!-- language + a close button in the corner (for desktop / people who don't know swipe or click-outside) -->
			<div class="menu-header-end">
				<nav class="nav nav--language" aria-label={m['menu.language']()}>
					<ul class="nav-list">
						{#each locales as locale}
							<li class="nav-item">
								<button
									type="button"
									class={`nav-btn ${getLocale() === locale ? 'active' : ''}`}
									aria-pressed={getLocale() === locale}
									lang={locale}
									onclick={() => setLocale(locale)}>{locale}</button
								>
							</li>
						{/each}
					</ul>
				</nav>
				<button type="button" class="menu-close" onclick={() => (closeMenu(), toggleElement?.focus())} aria-label={m['menu.close']()} data-testid="menu-close">
					<CloseIcon size="1rem" />
				</button>
			</div>
		</div>
		<!-- the toons, under account + language (owner 2026-10-01) -->
		<nav class="nav nav--toons" aria-label={m['menu.toons']()}>
			<ul class="nav-list">
				{#each toons as toon}
					<li class="nav-item">
						{#if toon.announced}
							<!-- announced, not published (owner 2026-10-01): greyed, "bald" / "soon", no link; the tile image, not the avatar -->
							<span class="nav-btn soon" data-control-exempt="toon avatar row" aria-disabled="true" data-testid="menu-toon-soon" data-toon={toon.id}>
								<img src={toonImage(toon)} alt="" />
								<span class="name">
									{toonName(toon, getLocale())}
									<span class="soon-label">{m['home.soon']()}</span>
								</span>
							</span>
						{:else}
							<a class="nav-btn" data-control-exempt="toon avatar row" href={localizeHref('/' + toon.id)} onclick={closeMenu} data-toon={toon.id}>
								<img src={asset(`${toon.id}/${toon.avatar}`)} alt="" />
								<span class="name">
									{toonName(toon, getLocale())}
								</span>
							</a>
						{/if}
					</li>
				{/each}
			</ul>
		</nav>
		<nav class="nav nav--home">
			<ul class="nav-list">
				{#if !isValid}
					<li class="nav-item">
						<a class="nav-btn sc-control" href={shopHref} onclick={closeMenu}>{m['account.buy_access']()}</a>
					</li>
				{/if}
				<li class="nav-item">
					<a class="nav-btn sc-control" href={shopHref} onclick={closeMenu}>{m.limited_editions()}</a>
				</li>
				{#if hasStores}
					<li class="nav-item">
						<a class="nav-btn" href={localizeHref('/stores')} onclick={closeMenu} data-testid="menu-stores">{m['menu.stores']()}</a>
					</li>
				{/if}
				<li class="nav-item">
					<a class="nav-btn sc-control" href={localizeHref('/events')} onclick={closeMenu}>{m.events()}</a>
				</li>
			</ul>
		</nav>
	</div>
</div>

<style>
	.main-menu {
		position: fixed;
		top: 0;
		left: 0;
		z-index: 1003;
		transition: transform 0.3s ease;
	}

	/* the round icon toggles of the top-right group (list, map, inventory: toon/*.svelte), right-most. One control
	   height (owner 2026-10-02): --control-height round, each next one --control-height + 0.5rem further left */
	.menu-toggle {
		position: fixed;
		top: 0.5rem;
		right: 0.5rem;
		/* under the open slide-out (1003), which then covers it */
		z-index: 1002;
		box-sizing: border-box;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-full);
		cursor: pointer;
		touch-action: manipulation;
		transition: all 0.2s ease;
	}
	.menu-toggle:hover {
		background: var(--primary-cyan);
	}
	.menu-toggle img {
		width: 1rem;
	}

	.logo-link {
		position: fixed;
		top: 0.5rem;
		left: 0;
		z-index: 1004;
		display: block;
		line-height: 0;
		touch-action: manipulation;
	}
	.logo-link img {
		height: 3rem;
	}
	.main-menu nav ul {
		margin: 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}
	.menu {
		padding: 1rem;
	}

	.main-menu .nav-btn:focus-visible,
	.logo-link:focus-visible,
	.menu-toggle:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: 2px;
	}

	.main-menu .nav-item {
		margin: 0;
		padding: 0;
	}

	.main-menu .nav-item .nav-btn {
		display: flex;
		flex: 1;
		align-items: center;
		gap: 0.5rem;
		height: var(--control-height);
		padding: 0 0.5rem;
		border: none;
		text-decoration: none;
		color: black;
		background: transparent;
		border-radius: 0;
		font-family: var(--font-mono);
		/* the button type of the design system: fluid root size (14 / 16 / 24 px), no px overrides */
		font-size: var(--type-button-size);
		line-height: var(--type-button-line);
		cursor: pointer;
	}

	.main-menu nav ul li.nav-item .nav-btn {
		box-sizing: border-box;
		display: flex;
		align-items: center;
		gap: 0.25rem;
		width: 100%;
	}

	.main-menu nav ul li.nav-item .nav-btn {
		border: 2px solid #ccc;
	}

	.main-menu nav ul li.nav-item .nav-btn:hover {
		border-color: black;
		text-decoration: none;
	}

	.main-menu nav.nav ul.nav-list li.nav-item .nav-btn img {
		width: 3rem;
	}
	/* a toon = a row with its 3rem avatar, not a single-line control */
	.main-menu nav.nav--toons ul.nav-list li.nav-item .nav-btn {
		height: 3rem; /* control-ok: the avatar row */
	}

	/* labels next to an icon (account, the toons) read like every other menu item */
	.main-menu nav.nav ul.nav-list li.nav-item .nav-btn span {
		font-size: inherit;
		line-height: inherit;
	}

	.main-menu nav.nav.nav--toons ul.nav-list li.nav-item .nav-btn:hover {
		text-decoration: underline;
	}

	.nav.nav--account {
		img {
			width: 2rem;
		}
		& li.nav-item .nav-btn {
			width: auto;
			border: none;
			padding: 0;
		}
	}

	/* the header row sits --space-05 from the top/right edge, like the menu toggle, so the close button lands
	   exactly where the toggle was (the rest of the menu keeps its --space-1 padding) */
	.menu-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin: calc(var(--space-05) - var(--space-1)) calc(var(--space-05) - var(--space-1)) var(--space-1) 0;
	}
	.menu-header-end {
		display: flex;
		align-items: center;
		gap: var(--space-05);
	}
	.main-menu .menu-header nav ul {
		margin-bottom: 0;
	}
	/* close: the round icon toggle of the top-right group (.menu-toggle) – a --control-height circle, regular frame,
	   paper, cyan on hover (owner 2026-10-04). The languages share the look but are a group, so square and joined
	   into one frame (owner 2026-10-07, DESIGN.md "Round vs square") */
	.menu-close,
	.main-menu nav.nav--language ul.nav-list li.nav-item .nav-btn {
		box-sizing: border-box;
		display: flex;
		align-items: center;
		justify-content: center;
		width: var(--control-height);
		height: var(--control-height);
		padding: 0;
		color: var(--ink);
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-full);
		cursor: pointer;
		touch-action: manipulation;
		transition: all 0.2s ease;
	}
	.main-menu nav.nav--language ul.nav-list li.nav-item .nav-btn {
		border-radius: var(--radius-none);
	}
	/* the chosen language inverted (DESIGN.md "Active / selected = inverted"), the others plain in the shared frame */
	.main-menu nav.nav--language ul.nav-list li.nav-item .nav-btn.active {
		background: var(--ink);
		color: var(--paper);
	}
	.menu-close:hover,
	.main-menu nav.nav--language ul.nav-list li.nav-item .nav-btn:hover {
		background: var(--primary-cyan);
		border-color: var(--ink);
	}
	.menu-close:focus-visible {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: 2px;
	}
	.nav.nav--account .nav-list {
		display: flex;
		flex-direction: row;
	}
	.nav.nav--language .nav-list {
		flex-direction: row;
		gap: 0;
	}
	/* one frame around the row: neighbours share their 2px edge */
	.nav.nav--language .nav-item + .nav-item {
		margin-left: calc(-1 * var(--border-width-regular));
	}

	.nav--toons .nav-item .nav-btn {
		border: 0;
	}
	/* an announced toon: greyed, not clickable */
	.main-menu nav.nav--toons .nav-item .nav-btn.soon {
		color: var(--text-muted);
		cursor: default;
	}
	.main-menu nav.nav--toons .nav-item .nav-btn.soon img {
		opacity: 0.5;
	}
	.main-menu nav.nav.nav--toons ul.nav-list li.nav-item .nav-btn.soon:hover {
		text-decoration: none;
	}
	.soon-label {
		margin-left: 0.5em;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}

	.menu {
		width: calc(100% - 5rem - 2px);
		position: fixed;
		right: 0;
		top: 0;
		bottom: 0;
		background: white;
		transition: transform 0.1s ease;
		border-left: 2px solid black;
		transform: translateX(100vw);
	}
	.menu.open {
		transform: translateX(0);
	}
	/* follows the finger while dragged to close; vertical scrolling inside the menu stays native */
	.menu {
		touch-action: pan-y;
	}
	.menu.dragging {
		transition: none;
	}
</style>
