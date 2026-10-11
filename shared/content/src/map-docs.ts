// The world map as many documents (DRAFT, agents/MAP-DOCUMENTS.md – owner 2026-10-12: "alle einen eigenen Doc, den man
// versionieren kann"): a collection (collection-docs.ts) – `map` keeps the map's own settings (size, layers), every place
// is `place:<id>`, every route `route:<id>`. Everything that reads the map still works on ONE `WorldMap`, composed from
// the documents (here for TS, `Domain\Content\WorldMapDocs` for PHP).
//
//   map            MapSettings   { width, height, layers, accepted_warnings? }
//   place:<id>     PlaceDoc      a MapPlace without its id (the key holds it)
//   route:<id>     RouteDoc      a MapPath without its id
import type { AcceptedWarnings } from './accepted.ts';
import {
	addressProblems,
	collectionDocOfPath,
	composeCollection,
	type CollectionDocProblem,
	type CollectionDocs,
	type CollectionSpec
} from './collection-docs.ts';
import { checkWorldMap, type MapPath, type MapPlace, type MapValidateOptions, type WorldMap } from './map.ts';

/** The world map's documents */
export const MAP_COLLECTION: CollectionSpec = {
	key: 'map',
	parts: [
		{ kind: 'place', field: 'locations' },
		{ kind: 'route', field: 'routes' }
	]
};

/** The document `map`: the world map's own settings */
export interface MapSettings extends AcceptedWarnings {
	/** world units = px of the terrain artwork; every position is in this space */
	width: number;
	height: number;
	/** drawn bottom → top */
	layers: WorldMap['layers'];
}

/** The document `place:<id>` */
export type PlaceDoc = Omit<MapPlace, 'id'> &
	AcceptedWarnings & {
		/** taken off the map (a tombstone: the id stays reserved, the history stays) */
		removed?: boolean;
	};

/** The document `route:<id>` */
export type RouteDoc = Omit<MapPath, 'id'> &
	AcceptedWarnings & {
		/** taken off the map (a tombstone) */
		removed?: boolean;
	};

/** The one WorldMap of the documents (the settings with defaults; places / routes that are not removed, creation order) */
export function composeWorldMap(docs: CollectionDocs): WorldMap {
	const map = composeCollection(MAP_COLLECTION, docs);
	return { ...map, width: map.width ?? 0, height: map.height ?? 0, layers: Array.isArray(map.layers) ? map.layers : [] } as WorldMap;
}

/** a composed map's path → the document and the path inside it (`['locations', 3, 'x']` → `place:<id>`, `['x']`) */
export const mapDocOfPath = (map: WorldMap, path: readonly (string | number)[]) => collectionDocOfPath(MAP_COLLECTION, map as unknown as Record<string, any>, path);

/**
 * The world map's problems across its documents: checkWorldMap of the composition (references between places and
 * routes – a route through a removed place points at an unknown one –, duplicate ids, positions inside the map's size),
 * each addressed to its document.
 */
export function checkMapDocs(docs: CollectionDocs, options: MapValidateOptions = {}): CollectionDocProblem[] {
	const map = composeWorldMap(docs);
	return addressProblems(MAP_COLLECTION, map as unknown as Record<string, any>, checkWorldMap(map, options));
}
