// the bundler's worker imports (Vite) – the package does not depend on vite's own types
declare module '@scutoons/live-core/worker?worker' {
	const Worker_: { new (): Worker };
	export default Worker_;
}
declare module '@scutoons/live-core/worklet?worker&url' {
	const url: string;
	export default url;
}
