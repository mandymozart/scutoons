// quic.ts (#438): the browser's WebTransport under deck-space's relay client (relay.rs, relay-web's wasm), which decides
// when to dial it and what goes on which lane; this moves the bytes. The WebSocket and the timer: client.ts.

/** an open (or opening) QUIC connection */
export interface QuicPipe {
	write(lane: 'stream' | 'datagram', text: string): void;
	close(): void;
}

export type QuicWord = 'opened' | 'closed' | 'stream' | 'datagram';
/** what the connection says; `data`: the bytes, or why it closed */
export type QuicEvents = (what: QuicWord, data?: Uint8Array | string) => void;
export type QuicDial = (url: string, hash: number[] | null, on: QuicEvents) => QuicPipe;

export interface QuicPlatform {
	quic?: QuicDial;
}

/** relay-web's `RelayClient`, its QUIC side */
export interface QuicCore {
	quic_opened(now: number): void;
	quic_closed(now: number, why: string): void;
	quic_stream(now: number, bytes: Uint8Array): void;
	quic_datagram(now: number, bytes: Uint8Array): void;
}

type Pipe = { readable: ReadableStream<Uint8Array>; writable: WritableStream<Uint8Array> };
interface WebTransportLike {
	ready: Promise<unknown>;
	closed: Promise<unknown>;
	datagrams: Pipe;
	createBidirectionalStream(): Promise<Pipe>;
	close(): void;
}
type WebTransportCtor = new (url: string, options?: { serverCertificateHashes: { algorithm: string; value: Uint8Array }[] }) => WebTransportLike;

async function drain(from: ReadableStream<Uint8Array>, on: QuicEvents, what: QuicWord): Promise<void> {
	const r = from.getReader();
	for (;;) {
		const { done, value } = await r.read();
		if (done) return;
		on(what, value);
	}
}

/** The browser's WebTransport; a browser without it (or a refused dial) reports it closed. */
export const webTransport: QuicDial = (url, hash, on) => {
	const encode = new TextEncoder();
	const out: { stream?: WritableStreamDefaultWriter<Uint8Array>; datagram?: WritableStreamDefaultWriter<Uint8Array> } = {};
	let wt: WebTransportLike | undefined;
	const open = async () => {
		const WT = (globalThis as { WebTransport?: WebTransportCtor }).WebTransport;
		if (!WT) throw new Error('no WebTransport here');
		wt = new WT(url, hash ? { serverCertificateHashes: [{ algorithm: 'sha-256', value: Uint8Array.from(hash) }] } : undefined);
		await wt.ready;
		const bi = await wt.createBidirectionalStream();
		out.stream = bi.writable.getWriter();
		out.datagram = wt.datagrams.writable.getWriter();
		on('opened');
		void drain(bi.readable, on, 'stream').catch(String);
		void drain(wt.datagrams.readable, on, 'datagram').catch(String);
		await wt.closed;
		return '';
	};
	void open().catch(String).then(on.bind(null, 'closed'));
	return {
		write: (lane, text) => void out[lane]?.write(encode.encode(text)).catch(String),
		close: () => {
			try {
				wt?.close();
			} catch {
				// a session not yet ready: its `ready` rejects and says so
			}
		}
	};
};

/** what `QuicLink` needs of the socket it sits beside */
export interface QuicHost {
	readonly core: QuicCore;
	now(): number;
	run(): void;
}

/** One client's QUIC connection: does the core's QUIC words and hands the core what the connection says. */
export class QuicLink {
	private pipe: QuicPipe | null = null;
	/** the dial that still counts: a word from an older one is dropped */
	private dialed = 0;

	constructor(
		private readonly host: QuicHost,
		private readonly dial: QuicDial = webTransport
	) {}

	/** one of the core's words; false: not a QUIC one */
	act(o: Record<string, never>): boolean {
		if ('dial_quic' in o) this.pipe = this.dial(o.dial_quic['url'], o.dial_quic['hash'], this.word.bind(this, ++this.dialed));
		else if ('stream' in o) this.pipe?.write('stream', o.stream);
		else if ('datagram' in o) this.pipe?.write('datagram', o.datagram);
		else if ('close_quic' in o) this.close();
		else return false;
		return true;
	}

	private word(dialed: number, what: QuicWord, data?: Uint8Array | string): void {
		if (this.dialed !== dialed) return;
		const { core } = this.host;
		const now = this.host.now();
		if (what === 'opened') core.quic_opened(now);
		else if (what === 'stream') core.quic_stream(now, data as Uint8Array);
		else if (what === 'datagram') core.quic_datagram(now, data as Uint8Array);
		else {
			this.pipe = null;
			core.quic_closed(now, String(data ?? ''));
		}
		this.host.run();
	}

	/** the core is done with it: it hears the close now, whatever the browser does later */
	private close(): void {
		this.dialed++;
		this.pipe?.close();
		this.pipe = null;
		this.host.core.quic_closed(this.host.now(), '');
	}
}
