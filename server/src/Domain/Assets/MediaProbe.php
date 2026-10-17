<?php
declare(strict_types=1);

namespace Scutoons\Domain\Assets;

use Scutoons\Infrastructure\Config\Settings;

/**
 * Size / duration of a local media file: images with getimagesize (no GD needed; SVG from width/height/viewBox),
 * video and audio with ffprobe when it is installed (FFPROBE_PATH). Never throws – unknown values stay null.
 */
final class MediaProbe
{
    private ?bool $ffprobe = null;

    public function __construct(private readonly Settings $settings)
    {
    }

    /** @return array{width: ?int, height: ?int, duration: ?float, bitrate: ?int} */
    public function probe(string $file, string $kind): array
    {
        $empty = ['width' => null, 'height' => null, 'duration' => null, 'bitrate' => null];
        if (!is_file($file)) return $empty;
        if ($kind === 'image') {
            // by content too: an upload's temp file has no extension
            $head = (string) file_get_contents($file, false, null, 0, 4096);
            if (str_ends_with(strtolower($file), '.svg') || preg_match('/^\s*(<\?xml[^>]*>\s*)?(<!--.*?-->\s*)*<svg\b/s', $head)) return [...$empty, ...self::svgSize($head)];
            $size = @getimagesize($file);
            return $size ? ['width' => (int) $size[0], 'height' => (int) $size[1], 'duration' => null, 'bitrate' => null] : $empty;
        }
        if (($kind === 'video' || $kind === 'audio') && $this->hasFfprobe()) return $this->ffprobe($file) + $empty;
        return $empty;
    }

    /** @return array{width: ?int, height: ?int} */
    public static function svgSize(string $head): array
    {
        if (!preg_match('/<svg\b[^>]*>/is', $head, $m)) return ['width' => null, 'height' => null];
        $tag = $m[0];
        $num = static fn(string $attr): ?int => preg_match('/\b' . $attr . '\s*=\s*["\']\s*([\d.]+)\s*(px)?\s*["\']/i', $tag, $a) ? (int) round((float) $a[1]) : null;
        $width = $num('width');
        $height = $num('height');
        if (($width === null || $height === null) && preg_match('/\bviewBox\s*=\s*["\']\s*[-\d.]+[\s,]+[-\d.]+[\s,]+([\d.]+)[\s,]+([\d.]+)/i', $tag, $v)) {
            $width ??= (int) round((float) $v[1]);
            $height ??= (int) round((float) $v[2]);
        }
        return ['width' => $width, 'height' => $height];
    }

    public function hasFfprobe(): bool
    {
        if ($this->ffprobe !== null) return $this->ffprobe;
        $out = [];
        $code = 1;
        @exec(escapeshellarg($this->settings->get('FFPROBE_PATH') ?: 'ffprobe') . ' -version 2>&1', $out, $code);
        return $this->ffprobe = $code === 0;
    }

    /** @return array{width: ?int, height: ?int, duration: ?float, bitrate: ?int} */
    private function ffprobe(string $file): array
    {
        $command = escapeshellarg($this->settings->get('FFPROBE_PATH') ?: 'ffprobe')
            . ' -v error -print_format json -show_entries format=duration,bit_rate:stream=codec_type,width,height ' . escapeshellarg($file) . ' 2>&1';
        $out = [];
        @exec($command, $out);
        $data = json_decode(implode("\n", $out), true);
        $result = ['width' => null, 'height' => null, 'duration' => null, 'bitrate' => null];
        if (!is_array($data)) return $result;
        foreach ((array) ($data['streams'] ?? []) as $stream) {
            if (is_array($stream) && ($stream['codec_type'] ?? '') === 'video' && isset($stream['width'])) {
                $result['width'] = (int) $stream['width'];
                $result['height'] = (int) ($stream['height'] ?? 0);
                break;
            }
        }
        $format = (array) ($data['format'] ?? []);
        if (isset($format['duration']) && is_numeric($format['duration'])) $result['duration'] = round((float) $format['duration'], 3);
        if (isset($format['bit_rate']) && is_numeric($format['bit_rate'])) $result['bitrate'] = (int) $format['bit_rate'];
        return $result;
    }
}
