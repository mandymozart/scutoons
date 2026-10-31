// The venue names (#201, agents/research/LIVE.md B.4): bunny Scriptable DNS on the zone lan.scutoons.com, attached as
// the wildcard `*` (a record of type Script). A deck's name is its LAN IP with dashes, so nothing is registered:
//   192-168-1-116.lan.scutoons.com  → A 192.168.1.116
// Only private addresses (RFC 1918: 10/8, 172.16/12, 192.168/16) – the names never point to the internet. Every other
// name, and every query type but A, gets no answer (an empty list). Explicit records win over the wildcard, so the
// TXT records acme.sh's dns_bunny adds for `_acme-challenge` (the workflow lan-cert.yml) work next to it.
// The runtime injects `ARecord` and allows no imports; paste this file into the zone's script editor as it is.
// Tested with node: deploy/test/lan-dns.test.mjs.

const ZONE = "lan.scutoons.com";
const TTL = 300;

/** "192-168-1-116.lan.scutoons.com" (any case, a trailing dot) → "192.168.1.116"; null when it is not one */
function lanAddress(hostname) {
	const name = String(hostname || "").toLowerCase().replace(/\.$/, "");
	if (!name.endsWith("." + ZONE)) return null;
	const label = name.slice(0, -(ZONE.length + 1));
	const m = /^(\d{1,3})-(\d{1,3})-(\d{1,3})-(\d{1,3})$/.exec(label);
	if (!m) return null;
	const o = m.slice(1).map(Number);
	if (m.slice(1).some((p) => p.length > 1 && p[0] === "0")) return null; // 010 is not 10
	if (o.some((n) => n > 255)) return null;
	const private1918 = o[0] === 10 || (o[0] === 172 && o[1] >= 16 && o[1] <= 31) || (o[0] === 192 && o[1] === 168);
	return private1918 ? o.join(".") : null;
}

export default function handleQuery(query) {
	const request = query && query.request ? query.request : {};
	const address = lanAddress(request.hostname);
	if (address === null || String(request.queryType || "A").toUpperCase() !== "A") return [];
	return new ARecord(address, TTL);
}
