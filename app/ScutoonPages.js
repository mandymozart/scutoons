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
		const matchResult = Router.findRoute(routes);
		console.log('Route found:', matchResult);
		
		if (matchResult) {
			this.currentPage = matchResult.route.path;
			this.loadPage(matchResult.route.component, matchResult.params);
		}
	}

	loadPage(component, params = {}) {
		const page = new component(this);
		page.init(params);
	}
}