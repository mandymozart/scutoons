// The boot screen's script (shared/boot – edit here, `npm run boot:sync` writes it into each app.html). Runs inline right
// after #sc-boot, before the app's code: draws the reader's page skeleton for the address (data-skeleton="reader"),
// "Patience …", the thin line for the app code loaded so far, and goes away as soon as the app has drawn something.
(() => {
	const boot = document.getElementById('sc-boot');
	const app = document.getElementById('sc-app');
	const bar = boot.querySelector('.bar');
	const fill = boot.querySelector('.fill');
	boot.querySelector('.text').textContent = document.documentElement.lang === 'de' ? 'Geduld …' : 'Patience …';

	// the reader: the page's shape by its address – a chapter = a strip of slices, a toon = cover, text, chapter rows;
	// anything else: the logo and the line alone
	if (boot.dataset.skeleton === 'reader') {
		const parts = location.pathname.split('/').filter(Boolean);
		if (parts[0] === 'de' || parts[0] === 'en') parts.shift();
		const other = ['account', 'attributions', 'contact', 'events', 'stores'];
		const kind = parts[1] === 'chapter' ? 'chapter' : parts.length === 1 && !other.includes(parts[0]) ? 'toon' : null;
		if (kind) {
			const line = (w) => `<i class="sk-line" style="width:${w}%"></i>`;
			const head = '<div class="sk-head"><i class="sk-logo"></i>' + '<i class="round"></i>'.repeat(5) + '</div>';
			const row = (a, b) => `<div class="sk-row"><i></i><span>${line(a)}${line(b)}</span></div>`;
			const body =
				kind === 'chapter'
					? '<div class="sk-strip">' + '<i class="sk-slice"></i>'.repeat(3) + '</div>'
					: '<i class="sk-cover"></i><div class="sk-body">' + line(92) + line(78) + line(55) + '<i class="sk-button"></i>' + row(70, 40) + row(62, 35) + '</div>';
			const sk = document.createElement('div');
			sk.className = 'sk';
			sk.setAttribute('aria-hidden', 'true');
			sk.innerHTML = head + body;
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
