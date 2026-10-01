<script lang="ts">
	import { setLocale, localizeHref, locales, getLocale } from '$lib/paraglide/runtime.js';
	import { tick } from 'svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { shopUrl } from '$lib/config.ts';
	import type { ToonIndex } from '@scutoons/content';
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

	let menuOpen = $state(false);
	let menuElement = $state<HTMLDivElement>();
	let toggleElement = $state<HTMLButtonElement>();

	async function toggleMenu(e?: MouseEvent | KeyboardEvent) {
		e?.stopPropagation();
		menuOpen = !menuOpen;
		if (menuOpen) {
			// keyboard users land on the first toon
			await tick();
			(menuElement?.querySelector<HTMLElement>('.nav--toons .nav-btn') ?? menuElement?.querySelector<HTMLElement>('.nav-btn'))?.focus();
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
					<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><path d="M3 3l10 10M13 3L3 13" fill="none" stroke="currentColor" stroke-width="2" /></svg>
				</button>
			</div>
		</div>
		<!-- the toons, under account + language (owner 2026-10-01) -->
		<nav class="nav nav--toons" aria-label={m['menu.toons']()}>
			<ul class="nav-list">
				{#each toons as toon}
					<li class="nav-item">
						<a class="nav-btn" href={localizeHref('/' + toon.id)} onclick={closeMenu}>
							<img src={asset(`${toon.id}/${toon.avatar}`)} alt="" />
							<span class="name">
								{toon.i18n[getLocale()].name}
							</span>
						</a>
					</li>
				{/each}
			</ul>
		</nav>
		<nav class="nav nav--home">
			<ul class="nav-list">
				{#if !isValid}
					<li class="nav-item">
						<a class="nav-btn" href={shopHref} onclick={closeMenu}>{m['account.buy_access']()}</a>
					</li>
				{/if}
				<li class="nav-item">
					<a class="nav-btn" href={shopHref} onclick={closeMenu}>{m.limited_editions()}</a>
				</li>
				<li class="nav-item">
					<a class="nav-btn" href={localizeHref('/events')} onclick={closeMenu}>{m.events()}</a>
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

	/* the round icon toggles of the top-right group (list, map, inventory: toon/*.svelte), right-most */
	.menu-toggle {
		position: fixed;
		top: 0.5rem;
		right: 0.5rem;
		/* under the open slide-out (1003), which then covers it */
		z-index: 1002;
		width: 2rem;
		height: 2rem;
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
		height: 3rem;
		padding: 0.25rem 0.5rem;
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

	.menu-header {
		display: flex;
		justify-content: space-between;
	}
	.menu-header-end {
		display: flex;
		align-items: flex-start;
	}
	/* a grey cross, the size of a language button, in the top-right corner */
	.menu-close {
		box-sizing: border-box;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 3rem;
		height: 3rem;
		padding: 0;
		background: transparent;
		color: var(--text-muted);
		border: 2px solid #ccc;
		cursor: pointer;
	}
	.menu-close:hover {
		color: black;
		border-color: black;
	}
	.menu-close svg {
		width: 1rem;
		height: 1rem;
	}
	.nav.nav--account .nav-list {
		display: flex;
		flex-direction: row;
	}
	/* the language buttons are squares (the account link keeps its icon + label width) */
	.nav.nav--language {
		.nav-list {
			display: flex;
			flex-direction: row;
			.nav-item .nav-btn {
				display: flex;
				width: 3rem;
				justify-content: center;
				&.active {
					border-color: black;
				}
			}
		}
	}

	.nav--toons .nav-item .nav-btn {
		border: 0;
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
