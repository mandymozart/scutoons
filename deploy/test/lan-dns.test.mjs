// The venue names' bunny DNS script (deploy/bunny/lan-dns.js, #201): <ip-with-dashes>.lan.scutoons.com → an A record,
// only for RFC 1918 addresses; nothing for every other name or type. bunny injects ARecord – a stand-in here.
//   node --test deploy/test/*.test.mjs
import assert from 'node:assert/strict';
import { test } from 'node:test';

globalThis.ARecord = class ARecord {
	constructor(address, ttl) {
		this.address = address;
		this.ttl = ttl;
	}
};
const { default: handleQuery } = await import('../bunny/lan-dns.js');

const ask = (hostname, queryType = 'A') => handleQuery({ request: { hostname, queryType, clientIP: '203.0.113.9' } });
const address = (hostname, queryType) => {
	const answer = ask(hostname, queryType);
	return Array.isArray(answer) ? (answer.length ? 'many' : null) : answer.address;
};

test('a private address with dashes answers its A record', () => {
	assert.equal(address('192-168-1-116.lan.scutoons.com'), '192.168.1.116');
	assert.equal(address('10-0-0-1.lan.scutoons.com'), '10.0.0.1');
	assert.equal(address('10-255-255-255.lan.scutoons.com'), '10.255.255.255');
	assert.equal(address('172-16-0-1.lan.scutoons.com'), '172.16.0.1');
	assert.equal(address('172-31-255-254.lan.scutoons.com'), '172.31.255.254');
	assert.equal(address('192-168-0-0.lan.scutoons.com'), '192.168.0.0');
	// resolvers send any case, often a trailing dot
	assert.equal(address('192-168-1-116.LAN.Scutoons.com.'), '192.168.1.116');
	assert.equal(ask('192-168-1-116.lan.scutoons.com').ttl, 300);
});

test('public and special addresses get nothing', () => {
	for (const ip of ['8-8-8-8', '172-15-0-1', '172-32-0-1', '192-169-1-1', '127-0-0-1', '169-254-1-1', '100-64-0-1', '0-0-0-0', '11-0-0-1']) {
		assert.equal(address(`${ip}.lan.scutoons.com`), null, ip);
	}
});

test('anything that is not four dashed octets gets nothing', () => {
	for (const name of [
		'lan.scutoons.com',
		'x1.lan.scutoons.com',
		'192-168-1.lan.scutoons.com',
		'192-168-1-116-1.lan.scutoons.com',
		'192-168-1-256.lan.scutoons.com',
		'192-168-001-116.lan.scutoons.com',
		'192.168.1.116.lan.scutoons.com',
		'a.192-168-1-116.lan.scutoons.com',
		'_acme-challenge.lan.scutoons.com',
		'192-168-1-116.lan.scutoons.com.evil.example',
		'192-168-1-116.scutoons.com',
		'',
	]) {
		assert.equal(address(name), null, name);
	}
	assert.deepEqual(handleQuery({}), []);
});

test('only A queries: AAAA, TXT … get nothing', () => {
	assert.equal(address('192-168-1-116.lan.scutoons.com', 'AAAA'), null);
	assert.equal(address('192-168-1-116.lan.scutoons.com', 'TXT'), null);
	assert.equal(address('192-168-1-116.lan.scutoons.com', 'a'), '192.168.1.116');
});
