// A tool's stored theme on <html> before anything paints (the creator's Settings › Theme, kept in this browser under
// the script's data-key; day when none), so the boot screen and the app's first frame stand on the stage of that
// theme – black by night, white by day, no white flash. `npm run boot:sync` writes it into the tool's app.html head.
(() => {
	const root = document.documentElement;
	let theme = 'day';
	try {
		const kept = JSON.parse(localStorage.getItem(document.currentScript.dataset.key) ?? 'null');
		if (kept && kept.theme === 'night') theme = 'night';
	} catch {
		/* no storage or a broken entry: day */
	}
	root.dataset.theme = theme;
	if (theme === 'night') document.querySelector('meta[name="theme-color"]')?.setAttribute('content', '#000000');
})();
