export class Router {
	static parsePath(path) {
		const queryIndex = path.indexOf('?');
		if (queryIndex !== -1) path = path.substring(0, queryIndex);
		if (path.endsWith('/') && path.length > 1) path = path.slice(0, -1);
		let segments = path.split('/').filter(seg => seg !== '');
		if (segments.length > 0) segments[segments.length - 1] = segments[segments.length - 1].replace(/\.html?$/, '');
		if (segments.length > 0 && segments[segments.length - 1] === 'index') segments.pop();
		return segments.join('/');
	}

	static matchRoute(path, routes) {
		for (const route of routes) if (route.path === path) return route;
		return null;
	}

	static findRoute(routes) {
		const path = Router.parsePath(window.location.pathname);
		return Router.matchRoute(path, routes);
	}
}
