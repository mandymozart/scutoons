import { HomePage } from './pages/home.js';
import { AttributionsPage } from './pages/attributions.js';

/**
 * Routes for the app
 * 
 * .html is omitted so just use name_of_page.html or /name_of_page/index.[html|php|etc] 
 * 
 * TODO: nesting and params are not working properly yet
 * 
 * @type {Array<{path: string, component: any}>}
 */
export const routes = [
	{
		path: '',
		component: HomePage
	},
	{
		path: 'attributions',
		component: AttributionsPage
	}
];