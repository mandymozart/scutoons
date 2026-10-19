// The handheld camera's link between the PC page and the phone page: the studio's dev-server socket (Vite's HMR
// WebSocket, custom event `hh` – handheld-relay.mjs) under a 4-digit pairing code. → { send(type, data), close() }
export function connect(code, from, onMessage) {
	const hot = import.meta.hot;
	if (!hot) throw new Error('the studio runs only in its dev server (npm run dev:studio)');
	const listener = (msg) => { if (msg?.code === code && msg.from !== from) onMessage(msg); };
	hot.on('hh', listener);
	const join = () => hot.send('hh', { code, from, type: 'join', ua: navigator.userAgent });
	join();
	// the socket reconnects after the PC slept or the dev server restarted: join again
	hot.on('vite:ws:connect', join);
	return {
		send(type, data = {}) { hot.send('hh', { ...data, code, from, type }); },
		close() { hot.off?.('hh', listener); hot.off?.('vite:ws:connect', join); }
	};
}

/** a new pairing code (4 digits) */
export const newCode = () => String(Math.floor(1000 + Math.random() * 9000));
