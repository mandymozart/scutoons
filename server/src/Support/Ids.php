<?php
declare(strict_types=1);

namespace Scutoons\Support;

/** Random ids: UUID v4, time-ordered UUID v7, nanoid-style QR codes */
final class Ids
{
    public const NANOID_ALPHABET = 'useandom-26T198340PX75pxJACKVERYMINDBUSHWOLF_GQZbfghjklqvwyzrict';

    public static function uuid(): string
    {
        $bytes = random_bytes(16);
        $bytes[6] = chr((ord($bytes[6]) & 0x0f) | 0x40);
        $bytes[8] = chr((ord($bytes[8]) & 0x3f) | 0x80);
        return vsprintf('%s%s-%s-%s-%s-%s%s%s', str_split(bin2hex($bytes), 4));
    }

    /**
     * Time-ordered UUID (version 7 layout: 48-bit Unix ms, then random) – sorts by creation time, also within one
     * second (login_requests: "the newest request of this address" = ORDER BY created_at DESC, id DESC).
     */
    public static function uuid7(?int $ms = null): string
    {
        $ms ??= (int) floor(microtime(true) * 1000);
        $bytes = substr(pack('J', $ms), 2, 6) . random_bytes(10);
        $bytes[6] = chr((ord($bytes[6]) & 0x0f) | 0x70);
        $bytes[8] = chr((ord($bytes[8]) & 0x3f) | 0x80);
        return vsprintf('%s%s-%s-%s-%s-%s%s%s', str_split(bin2hex($bytes), 4));
    }

    /** A random code like the generate SPA's nanoid(8): A-Z a-z 0-9 _ - */
    public static function nanoid(int $length = 8): string
    {
        $code = '';
        for ($i = 0; $i < $length; $i++) $code .= self::NANOID_ALPHABET[random_int(0, 63)];
        return $code;
    }

    /** URL-safe random token (32 bytes, base64url without padding) */
    public static function token(): string
    {
        return rtrim(strtr(base64_encode(random_bytes(32)), '+/', '-_'), '=');
    }
}
