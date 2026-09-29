<?php
declare(strict_types=1);

namespace Scutoons\Domain\Codes;

/** Rules on a `qr` row that are the same everywhere: landing page, validate, access, claim, /me */
final class CodeRules
{
    /** A syntactically valid code (generated codes are nanoid(8); up to 32 chars for hand-made test codes) or null */
    public static function normalize(mixed $value): ?string
    {
        if (!is_string($value)) return null;
        $code = trim($value);
        return preg_match('/^[A-Za-z0-9_-]{1,32}$/', $code) ? $code : null;
    }

    /**
     * The code has an expires_at in the past (stored in the server's timezone)
     * @param array<string, mixed> $row
     */
    public static function isExpired(array $row, ?int $now = null): bool
    {
        if (empty($row['expires_at'])) return false;
        $expires = strtotime((string) $row['expires_at']);
        return $expires !== false && $expires < ($now ?? time());
    }

    /**
     * An admin revoked the code (qr.revoked_at): it grants nothing any more
     * @param array<string, mixed> $row
     */
    public static function isRevoked(array $row): bool
    {
        return !empty($row['revoked_at']);
    }

    /**
     * Claimed by an account or by the old email-only form
     * @param array<string, mixed> $row
     */
    public static function isClaimed(array $row): bool
    {
        return $row['claimed_by'] !== null || (string) $row['email'] !== '';
    }
}
