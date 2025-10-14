
import { routes } from './routes.js';

export class ScutoonPages {
	lang;
	currentPage;

	constructor(lang) {
		this.lang = lang;
	}

	/** Call this explicitly */
	init() {
		console.log('ScutoonPages initialized');
		// Load page app by path
		const path = window.location.pathname;
		// Strip .html to get the page name
		const page = path.split('/').pop().replace('.html', '');
		this.currentPage = page;
		const route = routes.find((route) => route.path === this.currentPage);
		console.log('Route found:', route);
		if (route) {
			this.currentPage = route.path;
			this.loadPage(route.component);
		}
	}

	loadPage(component) {
		const page = new component();
		page.init(this);
	}
}