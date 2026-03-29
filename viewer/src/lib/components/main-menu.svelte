<script lang="ts">
	import { setLocale, localizeHref, locales, getLocale } from '$lib/paraglide/runtime.js';
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import type { ToonIndex } from '$lib/models/content.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { onClickOutside } from 'runed';
	import accountIcon from '$lib/icons/account.svg';

	let { toons }: { toons: ToonIndex[] } = $props();

	const membershipCtx = getMembershipContext();
	const membership = $derived(membershipCtx.membership);
	const isValid = $derived(membershipCtx.isValid);
	const isClaimed = $derived(isValid && !!membership?.email);

	let menuOpen = $state(false);
	let menuElement = $state<HTMLDivElement>();

	function toggleMenu(e?: MouseEvent | KeyboardEvent) {
		e?.stopPropagation();
		menuOpen = !menuOpen;
	}

	function closeMenu() {
		menuOpen = false;
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
			}
		}

		window.addEventListener('keydown', handleKeyDown);
		return () => window.removeEventListener('keydown', handleKeyDown);
	});

	const navigate = (
		url: string | URL,
		opts?: Parameters<typeof goto>[1]
	): ReturnType<typeof goto> => {
		menuOpen = false;
		return goto(url, opts);
	};
</script>

<div class="main-menu-toggle logo">
	<button class="btn-nav" onclick={toggleMenu}>
		<img src="/SCUTOONS-logo-animated.svg" alt="SCUTOONS" />
	</button>
</div>

<div class="main-menu">
	<div bind:this={menuElement} class={`menu ${menuOpen ? 'open' : 'closed'}`}>
		<div class="menu-header">
			<nav class="nav nav--account">
				<ul class="nav-list">
					<li class="nav-item">
						{#if isClaimed}
							<button class="nav-btn" onclick={() => navigate(localizeHref('/account'))}>
								<img src={accountIcon} alt={m['account.my_account']()} />
								<span>{m['account.my_account']()}</span>
							</button>
						{:else if isValid}
							<button class="nav-btn" onclick={() => navigate(localizeHref('/account'))}>
								<img src={accountIcon} alt={m['account.claim_membership']()} />
								<span>{m['account.claim_membership']()}</span>
							</button>
						{:else}
							<button class="nav-btn" onclick={() => navigate(localizeHref('/shop'))}>
								<img src={accountIcon} alt={m['account.buy_access']()} />
								<span>{m['account.buy_access']()}</span>
							</button>
						{/if}
					</li>
				</ul>
			</nav>
			<nav class="nav nav--language">
				<ul class="nav-list">
					{#each locales as locale}
						<li class="nav-item">
							<button
								class={`nav-btn ${getLocale() === locale ? 'active' : ''}`}
								onclick={() => setLocale(locale)}
								onkeydown={() => setLocale(locale)}>{locale}</button
							>
						</li>
					{/each}
				</ul>
			</nav>
		</div>
		<nav class="nav nav--home">
			<ul class="nav-list">
				<li class="nav-item">
					<button class="nav-btn" onclick={() => navigate(localizeHref('/'))}>SCUTOONS</button>
				</li>
				<li class="nav-item">
					<button class="nav-btn" onclick={() => navigate(localizeHref('/shop'))}
						>{m.limited_editions()}
					</button>
				</li>
				<li class="nav-item">
					<button class="nav-btn" onclick={() => navigate(localizeHref('/events'))}
						>{m.events()}</button
					>
				</li>
			</ul>
		</nav>
		<nav class="nav nav--toons">
			<ul class="nav-list">
				{#each toons as toon}
					<li class="nav-item">
						<button class="nav-btn" onclick={() => navigate(localizeHref('/' + toon.id))}>
							<img src={asset(`${toon.id}/${toon.avatar}`)} alt={toon.i18n[getLocale()].name} />
							<span class="name">
								{toon.i18n[getLocale()].name}
							</span>
						</button>
					</li>
				{/each}
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

	.main-menu-toggle {
		position: fixed;
		top: 0.5rem;
		left: 0;
		z-index: 1004;
	}
	.main-menu-toggle button {
		background: none;
		padding: 0;
		margin: 0;
		cursor: pointer;
		touch-action: manipulation;
	}
	.main-menu-toggle img {
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

	.main-menu .nav-item {
		margin: 0;
		padding: 0;
	}

	.main-menu .nav-item button {
		display: flex;
		flex: 1;
		align-items: center;
		gap: 0.5rem;
		font-size: 16px;
		line-height: 16px;
		height: 3rem;
		padding: 0.25rem 0.5rem;
		border: none;
		text-decoration: none;
		color: black;
		background: transparent;
		border-radius: 0;
		font-family: 'IBM Plex Mono', monospace;
		font-size: 1rem;
		cursor: pointer;
	}

	.main-menu nav ul li.nav-item button {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		width: 100%;
	}

	.main-menu nav ul li.nav-item button span {
		font-size: 10px;
	}

	.main-menu nav ul li.nav-item button.nav-btn {
		border: 2px solid #ccc;
	}

	.main-menu nav ul li.nav-item button.nav-btn:hover {
		border-color: black;
		text-decoration: none;
	}

	.main-menu nav.nav ul.nav-list li.nav-item button img {
		width: 3rem;
	}

	.main-menu nav.nav ul.nav-list li.nav-item button span {
		font-size: 16px;
		line-height: 16px;
	}

	.main-menu nav.nav.nav--toons ul.nav-list li.nav-item button:hover {
		text-decoration: underline;
	}

	.nav.nav--account {
		img {
			width: 2rem;
		}
		& li.nav-item button.nav-btn {
			width: auto;
			border: none;
			padding: 0;
		}
	}

	.menu-header {
		display: flex;
		justify-content: space-between;
	}
	.nav.nav--account,
	.nav.nav--language {
		.nav-list {
			display: flex;
			flex-direction: row;
			.nav-item button {
				display: flex;
				width: 3rem;
				justify-content: center;
				&.active {
					border-color: black;
				}
			}
		}
	}

	.nav--toons .nav-item button {
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
</style>
