// The boot screen's script (shared/boot – edit here, `npm run boot:sync` writes it into each app.html). Runs inline right
// after #sc-boot, before the app's code: draws the reader's page skeleton for the address (data-skeleton="reader"),
// "Patience …", the thin line for the app code loaded so far, and goes away as soon as the app has drawn something.
(() => {
	const boot = document.getElementById('sc-boot');
	const app = document.getElementById('sc-app');
	const bar = boot.querySelector('.bar');
	const fill = boot.querySelector('.fill');
	// the page loader's words (viewer messages common.loading)
	boot.querySelector('.mid .say').textContent = document.documentElement.lang === 'de' ? 'Geduld ...' : 'Patience ...';

	// the reader: its controls where the app draws them (the header on every toon page and chapter), on a toon page on a
	// phone also the description, the start button, the season heading and the chapter rows – only buttons and text,
	// no pictures (owner 2026-10-16: "deckungsgleich", "nur Buttons und Textblöcke"). Positions measured on the app
	// (375 px wide, root 14px): the cover is a square of the screen's width, the text starts 2 rem below it
	if (boot.dataset.skeleton === 'reader') {
		const parts = location.pathname.split('/').filter(Boolean);
		if (parts[0] === 'de' || parts[0] === 'en') parts.shift();
		const other = ['account', 'attributions', 'contact', 'events', 'stores'];
		const toon = parts.length === 1 && !other.includes(parts[0]);
		if (parts.length === 0) {
			// the landing page: the logo and the menu button, the masthead (wordmark + slogan over a rule), "TOONS" and the
			// tiles – a published toon full width with its Read button, then the coming ones two (desktop three) a row
			const tile = (first) =>
				`<i class="tile${first ? ' first' : ''}"><i class="pic"></i><span class="body"><span class="txt"><i class="name"></i><i class="meta"></i></span>${first ? '<i class="read"></i>' : ''}</span></i>`;
			const sk = document.createElement('div');
			sk.className = 'sk';
			sk.setAttribute('aria-hidden', 'true');
			sk.innerHTML =
				`<i class="logo"><img src="${boot.querySelector('.mid img').getAttribute('src')}" alt="" /></i><i class="round solo"></i>` +
				'<div class="hm"><div class="hm-sec hm-mast"><i class="wordmark"></i><i class="slogan"></i></div>' +
				`<div class="hm-sec hm-toons"><i class="heading"></i><div class="hm-tiles">${tile(true)}${tile(false)}${tile(false)}</div></div></div>`;
			boot.prepend(sk);
		} else if (toon || parts[1] === 'chapter') {
			const at = (cls, css) => `<i class="${cls}" style="${css}"></i>`;
			// below the cover: n rem down from its bottom edge
			const y = (rem) => `top:calc(100vw + ${rem} * var(--r))`;
			const x = (rem) => `left:calc(${rem} * var(--r))`;
			let html = `<i class="logo"><img src="${boot.querySelector('.mid img').getAttribute('src')}" alt="" /></i>`;
			for (let n = 0; n < 5; n++) html += at('round', `--n:${n}`);
			if (toon) {
				// the description: three lines of 1.3 rem
				html += at('page text', `${y(2.2)};${x(1)};width:calc(100vw - 2 * var(--r))`);
				html += at('page text', `${y(3.5)};${x(1)};width:calc(100vw - 2 * var(--r))`);
				html += at('page text', `${y(4.8)};${x(1)};width:calc(60vw - var(--r))`);
				html += at('page start', y(6.93));
				// the season heading, then the chapter rows (thumbnail, title, number on the right)
				html += at('page text', `${y(13.1)};${x(1)};height:calc(1.1 * var(--r));width:65vw`);
				for (let i = 0; i < 3; i++) {
					const top = 15.5 + i * 4.57;
					html += at('page thumb', y(top));
					html += at('page text', `${y(top + 0.8)};${x(7.5)};width:calc(${[10, 4.5, 3][i]} * var(--r))`);
					html += at('page text', `${y(top + 1.2)};right:var(--r);width:calc(${[3, 1.2, 1.2][i]} * var(--r));height:calc(0.75 * var(--r))`);
				}
			}
			const sk = document.createElement('div');
			sk.className = 'sk';
			sk.setAttribute('aria-hidden', 'true');
			sk.innerHTML = html;
			boot.prepend(sk);
		}
	}

	// the line: bytes of the app's own files (/_app/) arrived so far against the expected size (never full before the
	// app draws)
	let expect = Number(boot.dataset.expect) || 1000000;
	let loaded = 0;
	const show = () => {
		if (loaded > expect * 0.95) expect = loaded / 0.95;
		const share = Math.round((loaded / expect) * 100);
		fill.style.width = share + '%';
		bar.setAttribute('aria-valuenow', String(share));
	};
	let files = null;
	try {
		files = new PerformanceObserver((list) => {
			for (const entry of list.getEntries()) if (entry.name.includes('/_app/')) loaded += entry.decodedBodySize || entry.encodedBodySize || 0;
			show();
		});
		files.observe({ type: 'resource', buffered: true });
	} catch {
		/* no PerformanceObserver: the logo alone */
	}
	// away as soon as the app has drawn something (its first element that isn't a script) – but not while it only waits
	// (an element with data-sc-booting: the sign-in gate checking the session, the reader's own loader)
	const drawn = () => [...app.children].some((el) => el.tagName !== 'SCRIPT') && !app.querySelector('[data-sc-booting]');
	const done = () => {
		files?.disconnect();
		boot.remove();
	};
	if (drawn()) return done();
	const watch = new MutationObserver(() => {
		if (!drawn()) return;
		watch.disconnect();
		done();
	});
	watch.observe(app, { childList: true, subtree: true });
	// never stuck: whatever the app shows after 10 s (an API that doesn't answer) is better than the logo
	setTimeout(() => {
		watch.disconnect();
		if (boot.isConnected) done();
	}, 10000);
})();
