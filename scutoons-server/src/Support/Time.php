<?php
declare(strict_types=1);

namespace Scutoons\Support;

final class Time
{
    /** DATETIME stored in UTC (account tables) → ISO 8601, null for empty values */
    public static function iso(mixed $value): ?string
    {
        if (!is_string($value) || $value === '') return null;
        $time = strtotime($value . ' UTC');
        return $time === false ? null : gmdate('c', $time);
    }

    /** DATETIME stored in the server's timezone (`qr` table) → ISO 8601 with the server's offset, null for empty */
    public static function localIso(mixed $value): ?string
    {
        if (!is_string($value) || $value === '') return null;
        return date('c', (int) strtotime($value));
    }
}
