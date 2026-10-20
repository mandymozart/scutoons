<?php
declare(strict_types=1);

namespace Scutoons\Domain\Storyboard;

/**
 * An MP4's / MOV's length in seconds from its `moov` → `mvhd` box – no ffprobe (ported from the studio's media.mjs).
 * Runway's Aleph is priced by the control video's seconds.
 */
final class Mp4
{
    public static function seconds(string $path): ?float
    {
        $f = @fopen($path, 'rb');
        if ($f === false) return null;
        try {
            return self::walk($f, 0, (int) filesize($path), 0);
        } finally {
            fclose($f);
        }
    }

    /** @param resource $f */
    private static function walk($f, int $start, int $end, int $depth): ?float
    {
        $pos = $start;
        while ($pos + 8 <= $end && $depth < 4) {
            $h = self::read($f, $pos, 16);
            if (strlen($h) < 8) return null;
            $len = self::u32($h, 0);
            $type = substr($h, 4, 4);
            $head = 8;
            if ($len === 1) {
                if (strlen($h) < 16) return null;
                $len = self::u64($h, 8);
                $head = 16;
            } elseif ($len === 0) {
                $len = $end - $pos;
            }
            if ($len < 8) return null;
            if ($type === 'moov') return self::walk($f, $pos + $head, $pos + $len, $depth + 1);
            if ($type === 'mvhd') {
                $b = self::read($f, $pos + $head, 32);
                if (strlen($b) < 32) return null;
                $v = ord($b[0]);
                $scale = self::u32($b, $v === 1 ? 20 : 12);
                $duration = $v === 1 ? self::u64($b, 24) : self::u32($b, 16);
                return $scale > 0 ? $duration / $scale : null;
            }
            $pos += $len;
        }
        return null;
    }

    private static function u32(string $b, int $at): int
    {
        $v = unpack('N', substr($b, $at, 4));
        return is_array($v) ? (int) $v[1] : 0;
    }

    private static function u64(string $b, int $at): int
    {
        $v = unpack('J', substr($b, $at, 8));
        return is_array($v) ? (int) $v[1] : 0;
    }

    /**
     * @param resource $f
     * @param int<1, max> $n
     */
    private static function read($f, int $pos, int $n): string
    {
        if (fseek($f, $pos) !== 0) return '';
        return (string) fread($f, $n);
    }
}
