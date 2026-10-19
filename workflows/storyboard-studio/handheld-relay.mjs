// The handheld camera's relay (research/HANDHELD-CAMERA.md §4): the phone page and the PC page meet on the studio's own
// dev-server socket (Vite's HMR WebSocket, custom event `hh`) under a pairing code; every message is passed on to the
// other pages of the same code. No extra port, no dependency; the relay runs on the PC that shows the camera, so a pose
// takes one hop.
//
// Messages: { code, from: 'pc' | 'phone', type, … }. `join` registers the page; the others hear `peer` (+ `left` when
// it closes). Everything else is passed on as it is (pose, spec, origin, rec, state).

/** @param {import('vite').ViteDevServer} server */
export function handheldRelay(server) {
	/** code → Map<socket, { client, from }> */
	const rooms = new Map();
	const others = (code, socket) => [...(rooms.get(code) ?? new Map()).entries()].filter(([s]) => s !== socket).map(([, v]) => v);
	server.ws.on('hh', (msg, client) => {
		if (!msg || typeof msg.code !== 'string' || !/^\d{4}$/.test(msg.code)) return;
		const socket = client.socket ?? client;
		if (msg.type === 'join') {
			for (const room of rooms.values()) room.delete(socket); // one code per page
			if (!rooms.has(msg.code)) rooms.set(msg.code, new Map());
			rooms.get(msg.code).set(socket, { client, from: msg.from });
			socket.once?.('close', () => {
				const room = rooms.get(msg.code);
				if (!room?.delete(socket)) return;
				for (const o of others(msg.code, socket)) o.client.send('hh', { code: msg.code, from: msg.from, type: 'left' });
				if (!room.size) rooms.delete(msg.code);
			});
			for (const o of others(msg.code, socket)) {
				o.client.send('hh', { code: msg.code, from: msg.from, type: 'peer', ua: msg.ua ?? '' });
				client.send('hh', { code: msg.code, from: o.from, type: 'peer' });
			}
			return;
		}
		if (!rooms.get(msg.code)?.has(socket)) return;
		for (const o of others(msg.code, socket)) o.client.send('hh', msg);
	});
}
