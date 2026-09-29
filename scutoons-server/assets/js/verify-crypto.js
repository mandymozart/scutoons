/**
 * Post-Quantum Cryptography verification utilities
 * Implements ML-DSA (Dilithium) signature verification
 * Uses noble-post-quantum library
 * 
 * This module contains only verification functions (no signing/key generation)
 * Safe to expose publicly for certificate verification
 */

// Access ML-DSA from the global noblePostQuantum object
const getMlDsa = () => {
	if (typeof window.noblePostQuantum === 'undefined' || !window.noblePostQuantum.ml_dsa65) {
		throw new Error('noble-post-quantum library not loaded. Include noble-post-quantum.min.js before this script.');
	}
	return window.noblePostQuantum.ml_dsa65;
};

/**
 * Verify a signature
 * @param {string} signatureHex - Hex-encoded signature
 * @param {string} publicKeyHex - Hex-encoded public key
 * @param {string} signedData - The original signed data
 * @returns {boolean} True if signature is valid
 */
export function verifySignature(signatureHex, publicKeyHex, signedData) {
	try {
		const ml_dsa65 = getMlDsa();
		
		console.log('verifySignature called with:');
		console.log('- signatureHex length:', signatureHex.length, 'chars');
		console.log('- publicKeyHex length:', publicKeyHex.length, 'chars');
		console.log('- signedData length:', signedData.length, 'chars');
		
		const signature = hexToBytes(signatureHex);
		const publicKey = hexToBytes(publicKeyHex);
		const message = new TextEncoder().encode(signedData);
		
		console.log('After conversion:');
		console.log('- signature:', signature.length, 'bytes');
		console.log('- publicKey:', publicKey.length, 'bytes');
		console.log('- message:', message.length, 'bytes');
		
		// Validate lengths
		if (signature.length !== 3309) {
			console.warn('⚠️ Unexpected signature length! Expected 3309, got', signature.length);
		}
		if (publicKey.length !== 1952) {
			console.warn('⚠️ Unexpected public key length! Expected 1952, got', publicKey.length);
		}
		
		// ML-DSA verify: verify(signature, message, publicKey)
		console.log('Attempting verification...');
		console.log('Testing verify(signature, message, publicKey)...');
		const result = ml_dsa65.verify(signature, message, publicKey);
		console.log('Verification result:', result);
		return result;
	} catch (error) {
		console.error('Signature verification failed:', error);
		return false;
	}
}

/**
 * Import signature data from storage
 * @param {string} base64Data - Base64-encoded signature package
 * @returns {Object} Signature data
 */
export function importSignature(base64Data) {
	const json = atob(base64Data);
	return JSON.parse(json);
}

// Utility functions
function hexToBytes(hex) {
	const bytes = new Uint8Array(hex.length / 2);
	for (let i = 0; i < bytes.length; i++) {
		bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
	}
	return bytes;
}
