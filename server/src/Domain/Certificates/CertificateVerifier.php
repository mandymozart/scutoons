<?php
declare(strict_types=1);

namespace Scutoons\Domain\Certificates;

use Scutoons\Domain\ApiError;
use Scutoons\Domain\Codes\CodeRules;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Signing\KeyStore;
use Scutoons\Infrastructure\Signing\MlDsa65;

/**
 * Certificate verification (agents/ITEMS.md "Certificates"): is a certificate / SIGNATURE-<code>.json authentic?
 *
 * Authentic = the ML-DSA-65 signature verifies against OUR PUBLISHED public key (/api/keys/keys.php, never the key a
 * file brings along) AND it is the signature we stored for that code (qr.signature). Checked here, server-side. The
 * answer carries the code's public status (registered to an account yes/no, revoked yes/no) – never who owns it.
 *
 * Codes are case-sensitive and matched exactly (owner 2026-10-03), like everywhere else.
 *
 * Reasons (valid = false): unknown_code (also a code in the wrong case) · no_signature · key_mismatch (not signed with our key: not issued by SCUTOONS) · bad_signature (the signature does not
 * verify: tampered) · signature_mismatch (signed by us, but not the signature on record for that code).
 */
final class CertificateVerifier
{
    public function __construct(private readonly Database $db, private readonly KeyStore $keys)
    {
    }

    /**
     * Verify the certificate of a code as we stored it (the code exactly as written, or a pasted QR / verify URL)
     * @return array<string, mixed>
     */
    public function byCode(mixed $input): array
    {
        $code = self::normalizeCode($input) ?? throw new ApiError(400, 'invalid_code', 'Code missing or malformed.');
        $publicKey = $this->publicKey();
        [$row, $reason] = $this->resolve($code);
        if ($row === null) return self::result('code', false, $reason, null);
        [$stored, $reason] = $this->stored($row, $publicKey);
        return self::result('code', $reason === null, $reason, $row, $reason === null ? $stored : null);
    }

    /**
     * Verify an uploaded SIGNATURE-<code>.json ({ signature: { signature, publicKey, signedData, … }, … } or the inner
     * object alone) against our key and our records
     * @return array<string, mixed>
     */
    public function byFile(mixed $file): array
    {
        $signature = is_array($file) && is_array($file['signature'] ?? null) ? $file['signature'] : $file;
        if (!is_array($signature) || !is_string($signature['signature'] ?? null) || !is_string($signature['publicKey'] ?? null)
            || !is_string($signature['signedData'] ?? null)) {
            throw new ApiError(400, 'invalid_signature_file', 'Not a SCUTOONS signature file (signature, publicKey, signedData missing).');
        }
        $publicKey = $this->publicKey();
        $signed = json_decode($signature['signedData'], true);
        $code = self::normalizeCode(is_array($signed) ? ($signed['code'] ?? null) : null)
            ?? self::normalizeCode(is_array($file) ? ($file['code'] ?? null) : null);
        [$row, $lookupReason] = $code !== null ? $this->resolve($code) : [null, 'unknown_code'];
        [$stored, $storedReason] = $row !== null ? $this->stored($row, $publicKey) : [null, null];

        $reason = match (true) {
            strtolower($signature['publicKey']) !== strtolower($publicKey) => 'key_mismatch',
            !MlDsa65::verifyHex($publicKey, $signature['signedData'], $signature['signature']) => 'bad_signature',
            $row === null => $lookupReason,
            $storedReason === 'no_signature' => 'no_signature',
            $stored === null || $storedReason !== null => 'signature_mismatch',
            strtolower($stored['signature']) !== strtolower($signature['signature'])
                || $stored['signedData'] !== $signature['signedData'] => 'signature_mismatch',
            default => null,
        };
        return self::result('file', $reason === null, $reason, $row, $storedReason === null ? $stored : null);
    }

    /**
     * A typed / printed code: trimmed, inner spaces dropped, a pasted QR URL (?c=) or verify URL (?code=) reduced to
     * the code. Case is kept: codes are case-sensitive and matched exactly.
     */
    public static function normalizeCode(mixed $input): ?string
    {
        if (!is_string($input)) return null;
        $value = trim($input);
        if (str_contains($value, '?') || str_contains($value, '://')) {
            parse_str((string) parse_url($value, PHP_URL_QUERY), $query);
            $value = $query['c'] ?? $query['code'] ?? '';
            if (!is_string($value)) return null;
        }
        return CodeRules::normalize((string) preg_replace('/\s+/', '', $value));
    }

    /** The ML-DSA-65 public key we publish (/api/keys/keys.php), hex */
    private function publicKey(): string
    {
        $key = $this->keys->keypair()['publicKey'] ?? null;
        if (!is_string($key) || strlen($key) !== 2 * MlDsa65::PUBLIC_KEY_BYTES) {
            throw new ApiError(503, 'keys_missing', 'Certificate verification is not available right now.');
        }
        return $key;
    }

    /**
     * The code's row – exact match only: codes are case-sensitive (owner 2026-10-03: one rule everywhere, no
     * case-insensitive lookup). qr.code is utf8mb4_bin on MariaDB (migration 2026-10-03-code-case), BINARY on SQLite.
     * @return array{0: array<string, mixed>|null, 1: string|null}
     */
    private function resolve(string $code): array
    {
        $row = $this->db->one('SELECT * FROM qr WHERE code = ?', [$code]);
        return $row !== null ? [$row, null] : [null, 'unknown_code'];
    }

    /**
     * The stored signature of a code and whether it is authentic (our key, verifies, signed for this very code)
     * @param array<string, mixed> $row
     * @return array{0: array{signature: string, publicKey: string, signedData: string, algorithm: string, timestamp: mixed}|null, 1: string|null}
     */
    private function stored(array $row, string $publicKey): array
    {
        if (empty($row['signature'])) return [null, 'no_signature'];
        $data = json_decode((string) $row['signature'], true);
        if (!is_array($data) || !is_string($data['signature'] ?? null) || !is_string($data['publicKey'] ?? null) || !is_string($data['signedData'] ?? null)) {
            return [null, 'bad_signature'];
        }
        $stored = [
            'signature' => $data['signature'],
            'publicKey' => $data['publicKey'],
            'signedData' => $data['signedData'],
            'algorithm' => is_string($data['algorithm'] ?? null) ? $data['algorithm'] : 'ML-DSA-65',
            'timestamp' => $data['timestamp'] ?? null,
        ];
        if (strtolower($stored['publicKey']) !== strtolower($publicKey)) return [$stored, 'key_mismatch'];
        if (!MlDsa65::verifyHex($publicKey, $stored['signedData'], $stored['signature'])) return [$stored, 'bad_signature'];
        $signed = json_decode($stored['signedData'], true);
        if (!is_array($signed) || ($signed['code'] ?? null) !== $row['code']) return [$stored, 'signature_mismatch'];
        return [$stored, null];
    }

    /**
     * @param array<string, mixed>|null $row
     * @param array{signature: string, publicKey: string, signedData: string, algorithm: string, timestamp: mixed}|null $stored only when authentic
     * @return array<string, mixed>
     */
    private static function result(string $source, bool $valid, ?string $reason, ?array $row, ?array $stored = null): array
    {
        return [
            'status' => 'success',
            'valid' => $valid,
            'reason' => $valid ? null : $reason,
            'source' => $source,
            'code' => $row !== null ? (string) $row['code'] : null,
            'registered' => $row !== null ? CodeRules::isClaimed($row) : null,
            'revoked' => $row !== null ? CodeRules::isRevoked($row) : null,
            'certificate' => $stored !== null ? self::certificate($stored) : null,
        ];
    }

    /**
     * What our record says (the signed data of the stored, authentic signature) – the fields printed on the certificate
     * @param array{signature: string, publicKey: string, signedData: string, algorithm: string, timestamp: mixed} $stored
     * @return array<string, mixed>
     */
    private static function certificate(array $stored): array
    {
        $signed = (array) json_decode($stored['signedData'], true);
        $int = static fn(mixed $value): ?int => is_numeric($value) ? (int) $value : null;
        $string = static fn(mixed $value): ?string => is_string($value) && $value !== '' ? $value : null;
        return [
            'code' => $string($signed['code'] ?? null),
            'type' => $string($signed['type'] ?? null),
            'product_name' => $string($signed['productname'] ?? null),
            'edition_number' => $int($signed['edition_number'] ?? null),
            'edition_count' => $int($signed['edition_count'] ?? null),
            'creators' => $string($signed['creators'] ?? null),
            'created_at' => $string($signed['created_at'] ?? null),
            'signed_at' => $string($stored['timestamp']),
            'fingerprint' => self::fingerprint($stored['signature']),
            'algorithm' => $stored['algorithm'],
        ];
    }

    /** `ABCD:EF01:…` – the first 32 bytes of the signature, as printed on the certificate (admin crypto.ts fingerprint()) */
    public static function fingerprint(string $signatureHex): string
    {
        return strtoupper(implode(':', str_split(substr($signatureHex, 0, 64), 4)));
    }
}
