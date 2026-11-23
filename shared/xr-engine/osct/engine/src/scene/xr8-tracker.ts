// The real tracker: the 8th Wall engine's image tracking (`tracker.ts`, `xr8.ts`). Only the reader imports this –
// `prefetchXr8()` fetches the engine's files ahead into the HTTP cache.
export { ImageTracker } from "./tracker";
export { prefetchXr8 } from "./xr8";
