import { routes } from './routes.js';
import { Router } from './utils/router.js';

export class ScutoonPages {
	lang;
	currentPage;

	constructor(lang) {
		this.lang = lang;
	}

	/** Call this explicitly */
	init() {
		console.log('ScutoonPages initialized');
		
		// Find matching route using Router utility
		const route = Router.findRoute(routes);
		console.log('Route found:', route);
		
		if (route) {
			this.currentPage = route.path;
			this.loadPage(route.component);
		}
	}

	loadPage(component) {
		const page = new component(this);
		page.init();
	}
}