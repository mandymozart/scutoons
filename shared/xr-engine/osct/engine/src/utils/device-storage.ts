/**
 * Where the engine keeps its switches for this device (the debug overlay's camera lens, resolution and zoom request,
 * the scenes kept ready): the app passes its storage once at startup (`setDeviceStorage`) – the engine never reads the
 * browser's storage itself (RULES #32). Without one the switches hold while the page lives. Keys are the engine's
 * (`camera-lens`); the app chooses where and under which names they land.
 */
export interface DeviceStorage {
  /** The stored text, null when nothing is stored (or the storage is blocked) */
  get(key: string): string | null;
  /** Store the text; null removes it. A storage that refuses keeps nothing – the default holds */
  set(key: string, value: string | null): void;
}

const inMemory = (): DeviceStorage => {
  const values = new Map<string, string>();
  return {
    get: key => values.get(key) ?? null,
    set: (key, value) => (value === null ? values.delete(key) : values.set(key, value)),
  };
};

let storage: DeviceStorage = inMemory();

/** The app's storage for the engine's switches; null = in memory again (tests) */
export const setDeviceStorage = (next: DeviceStorage | null): void => {
  storage = next ?? inMemory();
};

/** A switch's stored text, null when unset or unreadable */
export const readSwitch = (key: string): string | null => {
  try {
    return storage.get(key);
  } catch {
    return null;
  }
};

/** Store a switch (null = back to its default); a failing storage changes nothing */
export const writeSwitch = (key: string, value: string | null): void => {
  try {
    storage.set(key, value);
  } catch {
    // storage blocked: the default stays
  }
};
