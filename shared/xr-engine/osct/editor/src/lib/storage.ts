/**
 * The editor's settings in this browser's localStorage, each under its own `osct-…` key (the widths, the open
 * sections, the stage's preview, …) – never the session, which is the API's HttpOnly cookie (#207, RULES #34).
 * Storage may be blocked – a private window, a site policy, a full
 * quota: a read then finds nothing and a write is dropped, so the setting holds only while the page lives. A JSON value
 * that does not parse reads as nothing stored; the caller checks the shape of what it gets.
 */

import type { DeviceStorage } from "@engine/utils/device-storage";

/** A key of the editor's storage – keys never change, or stored settings are lost */
export type StorageKey = `osct-${string}`;

/** The stored text, null when nothing is stored or storage is blocked */
export const readStored = (key: StorageKey): string | null => {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
};

/** Stores the text; null removes the key */
export const writeStored = (key: StorageKey, value: string | null): void => {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, value);
  } catch {
    // storage blocked or full: the value holds while the page lives
  }
};

/** The stored JSON value, undefined when nothing (or no JSON) is stored */
export const readStoredJson = (key: StorageKey): unknown => {
  const raw = readStored(key);
  if (raw === null) return undefined;
  try {
    return JSON.parse(raw);
  } catch {
    return undefined;
  }
};

/** Stores a value as JSON */
export const writeStoredJson = (key: StorageKey, value: unknown): void => writeStored(key, JSON.stringify(value));

/** The engine's switches for this device (`setDeviceStorage`) under `osct-<key>` – the keys the reader uses on this origin */
export const deviceStorage: DeviceStorage = {
  get: key => readStored(`osct-${key}`),
  set: (key, value) => writeStored(`osct-${key}`, value),
};
