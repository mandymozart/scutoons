/**
 * Addresses (content schema 5.0, #334): one small grammar for "where" in the logic – the place a `goto` takes the reader
 * to, and the node a scene's or the experience's rule acts on. Shaped after SCUTOONS' address grammar (a scheme, a colon,
 * a path), so one parser serves both projects and later schemes fit in without a new form:
 *
 *   scene:<sceneId>                 a scene of the experience – a `goto`'s target (the reader opens it like /scene/<id>)
 *   target:<targetId>               the root node of a target's entity – a scene or experience rule's show / hide / play …
 *   target:<targetId>/<node name>   the node of that name in the target's entity tree (the first of that name)
 *
 * address = scheme ":" id [ "/" rest ]; scheme = lower-case letters; id = letters, digits, `_` and `-` (at most 64, the
 * ids of the content); rest = the node's name, as written in the tree (any text, trimmed). Later schemes – SCUTOONS'
 * `toon:` and `chapter:`, a goto's `via` (an on-screen instruction, "turn the page") – are additions to
 * `ADDRESS_SCHEMES`, not a new syntax.
 */

/** The schemes an address can have today */
export const ADDRESS_SCHEMES = ["scene", "target"] as const;
export type AddressScheme = (typeof ADDRESS_SCHEMES)[number];

/** Where a `goto` can take the reader */
export const GOTO_SCHEMES = ["scene"] as const satisfies readonly AddressScheme[];
/** What a scene's or the experience's rule names a node by */
export const NODE_SCHEMES = ["target"] as const satisfies readonly AddressScheme[];

export type Address =
  | { scheme: "scene"; id: string }
  /** `node` = the node's name in the target's entity tree; "" = the entity's root node */
  | { scheme: "target"; id: string; node: string };

/** An id in an address: the content's ids (scenes, targets – the API's `ID_PATTERN`) */
export const ADDRESS_ID_PATTERN = /^[A-Za-z0-9_-]{1,64}$/;

const SHAPE = /^([a-z]+):([^/]*)(?:\/(.*))?$/s;

/** An address's parts, or null when it is none (unknown scheme, a bad id, a path where the scheme takes none) */
export const parseAddress = (value: unknown): Address | null => {
  if (typeof value !== "string") return null;
  const match = SHAPE.exec(value.trim());
  if (!match) return null;
  const [, scheme, id, rest] = match;
  if (!ADDRESS_ID_PATTERN.test(id)) return null;
  if (scheme === "scene") return rest === undefined ? { scheme, id } : null;
  if (scheme === "target") return { scheme, id, node: (rest ?? "").trim() };
  return null;
};

/** An address as text: `scene:cover`, `target:door/hint`, `target:door` */
export const formatAddress = (address: Address): string =>
  address.scheme === "target" ? `target:${address.id}${address.node ? `/${address.node}` : ""}` : `${address.scheme}:${address.id}`;

/** `scene:<id>` */
export const sceneAddress = (sceneId: string): string => `scene:${sceneId}`;

/** `target:<targetId>/<node name>` (no name: the entity's root node) */
export const nodeAddress = (targetId: string, node = ""): string => formatAddress({ scheme: "target", id: targetId, node });

/**
 * Why a value is no address of one of the `allowed` schemes (null when it is one) – the guard's, the build's and the
 * editor's sentence
 */
export const addressProblem = (value: unknown, allowed: readonly AddressScheme[]): string | null => {
  const forms = allowed.map(scheme => (scheme === "target" ? "target:<targetId>/<node name>" : `${scheme}:<id>`)).join(" or ");
  if (typeof value !== "string" || !value.trim()) return `expected an address (${forms}), got ${JSON.stringify(value)}`;
  const address = parseAddress(value);
  if (!address) return `"${value}" is not an address (${forms})`;
  if (!(allowed as readonly string[]).includes(address.scheme)) return `"${value}": a ${address.scheme} address does not fit here (${forms})`;
  return null;
};
