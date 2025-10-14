/**
 * Router utility for matching paths to routes
 */
export class Router {
	/**
	 * Parse a URL path into segments, handling trailing slashes and extensions
	 * @param {string} path - The URL pathname
	 * @returns {string} - The cleaned path (e.g., 'events/special/2024')
	 */
	static parsePath(path) {
		// Remove trailing slash
		if (path.endsWith('/') && path.length > 1) {
			path = path.slice(0, -1);
		}
		
		// Extract segments and clean them
		let segments = path.split('/').filter(seg => seg !== '');
		
		// Remove .html/.htm extensions from last segment
		if (segments.length > 0) {
			segments[segments.length - 1] = segments[segments.length - 1]
				.replace(/\.html?$/, '');
		}
		
		// If the last segment is 'index', remove it
		if (segments.length > 0 && segments[segments.length - 1] === 'index') {
			segments.pop();
		}
		
		// Build the full path from segments
		return segments.join('/');
	}

	/**
	 * Match a path to a route, supporting wildcards and parameters
	 * @param {string} path - The path to match (e.g., 'events/special/2024')
	 * @param {Array} routes - Array of route objects with {path, component}
	 * @returns {{route: object, params: object}|null}
	 */
	static matchRoute(path, routes) {
		for (const route of routes) {
			const routePath = route.path;
			
			// Exact match
			if (routePath === path) {
				return { route, params: {} };
			}
			
			// Wildcard match (e.g., 'events/*' matches 'events/special')
			if (routePath.includes('*')) {
				const routePattern = routePath.replace(/\*/g, '.*');
				const regex = new RegExp(`^${routePattern}$`);
				if (regex.test(path)) {
					// Extract the wildcard part
					const wildcardPath = path.substring(routePath.indexOf('*'));
					return { route, params: { wildcard: wildcardPath, fullPath: path } };
				}
			}
			
			// Parameter match (e.g., 'events/:category/:id')
			if (routePath.includes(':')) {
				const routeSegments = routePath.split('/');
				const pathSegments = path.split('/');
				
				if (routeSegments.length === pathSegments.length) {
					const params = {};
					let matches = true;
					
					for (let i = 0; i < routeSegments.length; i++) {
						if (routeSegments[i].startsWith(':')) {
							// Parameter segment
							const paramName = routeSegments[i].substring(1);
							params[paramName] = pathSegments[i];
						} else if (routeSegments[i] !== pathSegments[i]) {
							// Static segment doesn't match
							matches = false;
							break;
						}
					}
					
					if (matches) {
						return { route, params };
					}
				}
			}
		}
		
		return null;
	}

	/**
	 * Find a matching route for the current URL
	 * @param {Array} routes - Array of route objects
	 * @returns {{route: object, params: object}|null}
	 */
	static findRoute(routes) {
		const path = Router.parsePath(window.location.pathname);
		return Router.matchRoute(path, routes);
	}
}
