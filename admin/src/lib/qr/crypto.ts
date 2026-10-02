// Post-quantum signatures of QR data (ML-DSA-65 / Dilithium), port of generate/app/utils/crypto.js on the npm
// @noble/post-quantum (compatible with the bundled copy the verify page uses: both sign/verify FIPS 204 ML-DSA-65).
import { ml_dsa65 } from '@noble/post-quantum/ml-dsa.js';
import type { CodeSignature } from '@scutoons/api';
import { admin } from '$lib/api.ts';

type Keypair = { secretKey: Uint8Array; publicKey: Uint8Array };
let keypair: Promise<Keypair> | null = null;

const hexToBytes = (hex: string) => {
	const bytes = new Uint8Array(hex.length / 2);
	for (let i = 0; i < bytes.length; i++) bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
	return bytes;
};
const bytesToHex = (bytes: Uint8Array) => Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');

/** The server keypair (GET /admin/keys), loaded once per page load */
export function loadKeypair(): Promise<Keypair> {
	keypair ??= admin
		.keys()
		.then((data) => {
			const secretKey = hexToBytes(data.privateKey);
			const publicKey = hexToBytes(data.publicKey);
			if (secretKey.length !== 4032) throw new Error(`Invalid secret key length: ${secretKey.length} (expected 4032)`);
			if (publicKey.length !== 1952) throw new Error(`Invalid public key length: ${publicKey.length} (expected 1952)`);
			return { secretKey, publicKey };
		})
		.catch((error) => {
			keypair = null;
			throw error;
		});
	return keypair;
}

/** Signs `JSON.stringify(qrData)` – every field, in insertion order (it is the `signedData` the verify page checks) */
export async function signQrData(qrData: Record<string, unknown>): Promise<CodeSignature> {
	const { secretKey, publicKey } = await loadKeypair();
	const signedData = JSON.stringify(qrData);
	const signature = ml_dsa65.sign(new TextEncoder().encode(signedData), secretKey);
	return {
		signature: bytesToHex(signature),
		publicKey: bytesToHex(publicKey),
		algorithm: 'ML-DSA-65',
		signedData,
		timestamp: new Date().toISOString()
	};
}

/** `ABCD:EF01:…` – the first 32 bytes of the signature, as printed on certificates */
export function fingerprint(signature: CodeSignature): string {
	return (signature.signature.slice(0, 64).match(/.{1,4}/g) ?? []).join(':').toUpperCase();
}

/**
 * Verifies a stored signature against the SERVER's public key (the keypair of GET /admin/keys = the key published at
 * /api/keys/keys.php) – never against the key embedded in the signature JSON: anyone can sign data with their own key
 * and embed that. With `code`, the signed data must also be for that code. (The public check is the API's
 * GET /api/certificates/{code}, agents/ITEMS.md "Certificates".)
 */
export async function verifySignature(signature: CodeSignature, code?: string): Promise<boolean> {
	try {
		const { publicKey } = await loadKeypair();
		if (signature.publicKey.toLowerCase() !== bytesToHex(publicKey)) return false;
		if (code !== undefined && (JSON.parse(signature.signedData) as { code?: unknown }).code !== code) return false;
		return ml_dsa65.verify(hexToBytes(signature.signature), new TextEncoder().encode(signature.signedData), publicKey);
	} catch {
		return false;
	}
}
