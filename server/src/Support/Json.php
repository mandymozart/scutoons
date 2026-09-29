<?php
declare(strict_types=1);

namespace Scutoons\Support;

final class Json
{
    /** The flags of every API response */
    public const FLAGS = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE;

    public static function encode(mixed $data, int $flags = self::FLAGS): string
    {
        $json = json_encode($data, $flags);
        if ($json === false) throw new \RuntimeException('JSON encoding failed: ' . json_last_error_msg());
        return $json;
    }

    /**
     * A JSON file decoded as arrays
     * @return array<mixed>
     */
    public static function file(string $path): array
    {
        $data = is_file($path) ? json_decode((string) file_get_contents($path), true) : null;
        if (!is_array($data)) throw new \RuntimeException('JSON file is missing or invalid: ' . $path);
        return $data;
    }
}
