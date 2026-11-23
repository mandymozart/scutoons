/**
 * Content on the CDN (docs/content-cdn.md): one folder per environment, one immutable folder per content version.
 *
 *   <env>/latest.json                    ContentManifest – the current version (the only file that is overwritten)
 *   <env>/versions.json                  ContentVersion[] – every version on the storage, for `osct content purge`
 *   <env>/<build>/game.config.json       the game configuration of that version
 *   <env>/<build>/files.json             ContentFiles – size and checksum of every content file
 *   <env>/<build>/assets/content/…       the content files, same paths as in the app's public folder
 *
 * `<build>` = the first 12 hex digits of the content build checksum (`ConfigurationVersion.hash`): the same content
 * is the same folder, whatever app version built it.
 *
 * An environment serves one experience – the last one published there; `experience` names it. A manifest without
 * `experience` was written before it existed: its version's `game.config.json` (`experience.id`) tells.
 */

import { isObject } from "../guards/primitives";

export interface ContentManifest {
  /** Folder of the current version below the environment */
  build: string;
  /** Content build checksum (`ConfigurationVersion.hash`) */
  hash: string;
  /** Content schema `MAJOR.MINOR` of that version */
  schema: string;
  /** App version that built it – information only */
  version: string;
  /** When this version became current (ISO 8601) */
  activatedAt: string;
  /** The experience this version is of (`ExperienceData.id`) */
  experience?: string;
}

/** Entry of versions.json */
export interface ContentVersion {
  build: string;
  hash: string;
  schema: string;
  version: string;
  uploadedAt: string;
  /** Last time it became current */
  activatedAt: string;
  /** When another version replaced it; null while it is current */
  retiredAt: string | null;
  /** The experience this version is of (`ExperienceData.id`) */
  experience?: string;
}

/** files.json of a version: by app path (`/assets/content/…`) */
export type ContentFiles = Record<string, { size: number; v: string }>;

/** A version folder name: 12 lower-case hex digits */
export const CONTENT_BUILD = /^[0-9a-f]{12}$/;

export const contentBuildName = (hash: string): string => hash.slice(0, 12).toLowerCase();

export const isContentManifest = (value: unknown): value is ContentManifest =>
  isObject(value) &&
  typeof value.build === "string" && CONTENT_BUILD.test(value.build) &&
  typeof value.hash === "string" &&
  typeof value.schema === "string" &&
  typeof value.version === "string" &&
  typeof value.activatedAt === "string" &&
  (value.experience === undefined || typeof value.experience === "string");

export const isContentVersions = (value: unknown): value is ContentVersion[] =>
  Array.isArray(value) &&
  value.every(v =>
    isObject(v) && typeof v.build === "string" && CONTENT_BUILD.test(v.build) && typeof v.hash === "string" &&
    typeof v.activatedAt === "string" && (v.retiredAt === null || typeof v.retiredAt === "string"));

export const isContentFiles = (value: unknown): value is ContentFiles =>
  isObject(value) &&
  Object.values(value).every(f => isObject(f) && typeof f.size === "number" && typeof f.v === "string");
