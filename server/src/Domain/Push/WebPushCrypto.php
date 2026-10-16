<?php
declare(strict_types=1);

namespace Scutoons\Domain\Push;

use OpenSSLAsymmetricKey;
use RuntimeException;

/**
 * The two pieces of crypto Web Push needs (agents/research/PWA.md §2b), on ext-openssl only – no composer package:
 * - VAPID (RFC 8292): an ES256 JWT { aud: the push service's origin, exp, sub } signed with the server's P-256 key,
 *   sent as `Authorization: vapid t=<jwt>, k=<public key>`;
 * - the payload encryption (RFC 8291 + RFC 8188 `aes128gcm`): ECDH with the browser's key, HKDF, AES-128-GCM, one record.
 * Keys are base64url: the public key = the uncompressed point (65 bytes), the private key = d (32 bytes) – the format of
 * `npx web-push generate-vapid-keys` and of `php bin/console push:vapid-keys`. Tested against RFC 8291's example.
 */
final class WebPushCrypto
{
    /** RFC 8188 record size; one record is all a push payload (≤ 4 KB) needs */
    private const RECORD_SIZE = 4096;

    public static function b64(string $bytes): string
    {
        return rtrim(strtr(base64_encode($bytes), '+/', '-_'), '=');
    }

    public static function unb64(string $text): string
    {
        $raw = base64_decode(strtr($text, '-_', '+/') . str_repeat('=', (4 - strlen($text) % 4) % 4), true);
        if ($raw === false) throw new RuntimeException('Not base64url.');
        return $raw;
    }

    /** @return array{public: string, private: string} a new P-256 key pair, base64url (the VAPID keys) */
    public static function generateKeys(): array
    {
        $key = self::newKey();
        [$public, $private] = self::raw($key);
        return ['public' => self::b64($public), 'private' => self::b64($private)];
    }

    /**
     * The encrypted body (`Content-Encoding: aes128gcm`) for one subscription
     * @param string $userPublic the subscription's `keys.p256dh` (base64url, 65 bytes)
     * @param string $authSecret the subscription's `keys.auth` (base64url, 16 bytes)
     * @param string|null $serverPrivate / $salt only for the RFC test vector – else a fresh key pair and salt per message
     */
    public static function encrypt(string $plaintext, string $userPublic, string $authSecret, ?string $serverPrivate = null, ?string $serverPublic = null, ?string $salt = null): string
    {
        $ua = self::unb64($userPublic);
        $auth = self::unb64($authSecret);
        if (strlen($ua) !== 65 || $ua[0] !== "\x04") throw new RuntimeException('Bad p256dh key.');
        if (strlen($auth) !== 16) throw new RuntimeException('Bad auth secret.');
        if ($serverPrivate !== null && $serverPublic !== null) {
            $asPublic = self::unb64($serverPublic);
            $key = self::privateKey(self::unb64($serverPrivate), $asPublic);
        } else {
            $key = self::newKey();
            [$asPublic] = self::raw($key);
        }
        $salt ??= random_bytes(16);
        $secret = openssl_pkey_derive(self::publicKey($ua), $key, 32);
        if ($secret === false) throw new RuntimeException('ECDH failed.');
        // RFC 8291 §3.4: IKM from the ECDH secret, keyed by the auth secret, bound to both public keys
        $ikm = hash_hkdf('sha256', $secret, 32, "WebPush: info\x00" . $ua . $asPublic, $auth);
        // RFC 8188 §2.2 / 2.3
        $cek = hash_hkdf('sha256', $ikm, 16, "Content-Encoding: aes128gcm\x00", $salt);
        $nonce = hash_hkdf('sha256', $ikm, 12, "Content-Encoding: nonce\x00", $salt);
        // the last (and only) record: the data, the delimiter 0x02, no padding
        $tag = '';
        $cipher = openssl_encrypt($plaintext . "\x02", 'aes-128-gcm', $cek, OPENSSL_RAW_DATA, $nonce, $tag, '', 16);
        if ($cipher === false) throw new RuntimeException('AES-GCM failed.');
        return $salt . pack('N', self::RECORD_SIZE) . chr(strlen($asPublic)) . $asPublic . $cipher . $tag;
    }

    /**
     * The VAPID Authorization header for a push service endpoint
     * @param string $publicKey / $privateKey base64url (VAPID_PUBLIC_KEY / VAPID_PRIVATE_KEY)
     * @param string $subject a mailto: or https: contact (VAPID_SUBJECT)
     */
    public static function vapidHeader(string $endpoint, string $publicKey, string $privateKey, string $subject, int $expires): string
    {
        $parts = parse_url($endpoint);
        if (!is_array($parts) || !isset($parts['scheme'], $parts['host'])) throw new RuntimeException('Bad endpoint.');
        $audience = $parts['scheme'] . '://' . $parts['host'] . (isset($parts['port']) ? ':' . $parts['port'] : '');
        $input = self::b64((string) json_encode(['typ' => 'JWT', 'alg' => 'ES256']))
            . '.' . self::b64((string) json_encode(['aud' => $audience, 'exp' => $expires, 'sub' => $subject], JSON_UNESCAPED_SLASHES));
        $public = self::unb64($publicKey);
        $der = '';
        if (!openssl_sign($input, $der, self::privateKey(self::unb64($privateKey), $public), OPENSSL_ALGO_SHA256)) {
            throw new RuntimeException('VAPID signing failed.');
        }
        return 'vapid t=' . $input . '.' . self::b64(self::derToRaw($der)) . ', k=' . $publicKey;
    }

    /** the keys are usable: a 65-byte point and a 32-byte scalar that belong together */
    public static function validKeys(string $publicKey, string $privateKey): bool
    {
        try {
            $public = self::unb64($publicKey);
            $private = self::unb64($privateKey);
            if (strlen($public) !== 65 || strlen($private) !== 32) return false;
            // the DER carries the point as given – only a signature shows that d belongs to it
            $signature = '';
            if (!openssl_sign('vapid', $signature, self::privateKey($private, $public), OPENSSL_ALGO_SHA256)) return false;
            return openssl_verify('vapid', $signature, self::publicKey($public), OPENSSL_ALGO_SHA256) === 1;
        } catch (RuntimeException) {
            return false;
        }
    }

    // ── keys ─────────────────────────────────────────────────────────────────────────────────────────────

    private static function newKey(): OpenSSLAsymmetricKey
    {
        $options = ['curve_name' => 'prime256v1', 'private_key_type' => OPENSSL_KEYTYPE_EC];
        $key = openssl_pkey_new($options);
        if ($key === false) {
            // Windows PHP without an openssl.cnf: an empty config file is enough for an EC key
            $config = (string) tempnam(sys_get_temp_dir(), 'sc-ossl');
            file_put_contents($config, '');
            $key = openssl_pkey_new($options + ['config' => $config, 'private_key_bits' => 2048]);
            @unlink($config);
        }
        while (openssl_error_string() !== false);
        if ($key === false) throw new RuntimeException('Cannot create an EC key.');
        return $key;
    }

    /** @return array{0: string, 1: string} the uncompressed public point (65 bytes) and d (32 bytes) */
    private static function raw(OpenSSLAsymmetricKey $key): array
    {
        $ec = openssl_pkey_get_details($key)['ec'] ?? null;
        if (!is_array($ec)) throw new RuntimeException('Not an EC key.');
        return ["\x04" . self::pad($ec['x']) . self::pad($ec['y']), self::pad($ec['d'])];
    }

    private static function pad(string $bytes): string
    {
        return str_pad($bytes, 32, "\x00", STR_PAD_LEFT);
    }

    /** SEC 1 ECPrivateKey (P-256) from d + the public point */
    private static function privateKey(string $d, string $public): OpenSSLAsymmetricKey
    {
        if (strlen($d) !== 32 || strlen($public) !== 65) throw new RuntimeException('Bad EC key.');
        $der = "\x30\x77\x02\x01\x01\x04\x20" . $d . "\xa0\x0a\x06\x08\x2a\x86\x48\xce\x3d\x03\x01\x07\xa1\x44\x03\x42\x00" . $public;
        $key = openssl_pkey_get_private(self::pem('EC PRIVATE KEY', $der));
        if ($key === false) throw new RuntimeException('Bad EC private key.');
        return $key;
    }

    /** SubjectPublicKeyInfo (P-256) from the uncompressed point */
    private static function publicKey(string $point): OpenSSLAsymmetricKey
    {
        $der = "\x30\x59\x30\x13\x06\x07\x2a\x86\x48\xce\x3d\x02\x01\x06\x08\x2a\x86\x48\xce\x3d\x03\x01\x07\x03\x42\x00" . $point;
        $key = openssl_pkey_get_public(self::pem('PUBLIC KEY', $der));
        if ($key === false) throw new RuntimeException('Bad EC public key.');
        return $key;
    }

    private static function pem(string $label, string $der): string
    {
        return "-----BEGIN $label-----\n" . chunk_split(base64_encode($der), 64, "\n") . "-----END $label-----\n";
    }

    /** an ECDSA signature: DER SEQUENCE { r, s } → JOSE's r || s (32 + 32 bytes) */
    private static function derToRaw(string $der): string
    {
        $offset = 2 + (ord($der[1]) & 0x80 ? ord($der[1]) & 0x7f : 0);
        $out = '';
        for ($i = 0; $i < 2; $i++) {
            if (($der[$offset] ?? '') !== "\x02") throw new RuntimeException('Bad ECDSA signature.');
            $length = ord($der[$offset + 1]);
            $int = ltrim(substr($der, $offset + 2, $length), "\x00");
            $out .= self::pad($int);
            $offset += 2 + $length;
        }
        return $out;
    }
}
