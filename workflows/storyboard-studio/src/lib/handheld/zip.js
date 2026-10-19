// A zip without compression (the downloads' bundle: GLB + JSON per shot). files: [{ name, data: Uint8Array | string }] → Blob
const CRC = new Uint32Array(256).map((_, n) => { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; return c >>> 0; });
const crc32 = (b) => { let c = 0xffffffff; for (let i = 0; i < b.length; i++) c = CRC[(c ^ b[i]) & 0xff] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; };

export function zip(files) {
	const enc = new TextEncoder();
	const parts = [], central = [];
	let offset = 0;
	for (const f of files) {
		const name = enc.encode(f.name);
		const data = typeof f.data === 'string' ? enc.encode(f.data) : f.data;
		const crc = crc32(data);
		const head = (sig, extra) => { const b = new DataView(new ArrayBuffer(extra)); b.setUint32(0, sig, true); return b; };
		const local = head(0x04034b50, 30);
		local.setUint16(4, 20, true); local.setUint16(6, 0x0800, true); local.setUint16(12, 33, true); // version, UTF-8 names, 1980-01-01
		local.setUint32(14, crc, true); local.setUint32(18, data.length, true); local.setUint32(22, data.length, true);
		local.setUint16(26, name.length, true);
		const dir = head(0x02014b50, 46);
		dir.setUint16(4, 20, true); dir.setUint16(6, 20, true); dir.setUint16(8, 0x0800, true); dir.setUint16(14, 33, true);
		dir.setUint32(16, crc, true); dir.setUint32(20, data.length, true); dir.setUint32(24, data.length, true);
		dir.setUint16(28, name.length, true); dir.setUint32(42, offset, true);
		parts.push(local, name, data);
		central.push(dir, name);
		offset += 30 + name.length + data.length;
	}
	const size = central.reduce((n, p) => n + p.byteLength, 0);
	const end = new DataView(new ArrayBuffer(22));
	end.setUint32(0, 0x06054b50, true);
	end.setUint16(8, files.length, true); end.setUint16(10, files.length, true);
	end.setUint32(12, size, true); end.setUint32(16, offset, true);
	return new Blob([...parts, ...central, end], { type: 'application/zip' });
}

/** hand a file to the browser's downloads */
export function download(name, blob) {
	const a = document.createElement('a');
	a.href = URL.createObjectURL(blob);
	a.download = name;
	a.click();
	setTimeout(() => URL.revokeObjectURL(a.href), 2000);
}
