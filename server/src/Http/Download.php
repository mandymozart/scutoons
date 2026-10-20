<?php
declare(strict_types=1);

namespace Scutoons\Http;

use Psr\Http\Message\ResponseInterface as Response;
use RuntimeException;
use Slim\Psr7\Stream;

/** File responses: CSV exports and stored files (ZIP, PDF, SVG, JSON) as attachments */
final class Download
{
    /** @param list<array<string, mixed>> $rows CSV with a header row; booleans as yes/no */
    public static function csv(Response $response, array $rows, string $filename): Response
    {
        $out = fopen('php://temp', 'w+');
        if ($out === false) throw new RuntimeException('Cannot open a temp stream');
        fputcsv($out, array_keys($rows[0] ?? ['code' => '']), ',', '"', '\\');
        foreach ($rows as $row) {
            fputcsv($out, array_map(fn($v) => is_bool($v) ? ($v ? 'yes' : 'no') : (is_scalar($v) || $v === null ? $v : json_encode($v)), $row), ',', '"', '\\');
        }
        rewind($out);
        return $response->withBody(new Stream($out))
            ->withHeader('Content-Type', 'text/csv; charset=utf-8')
            ->withHeader('Content-Disposition', 'attachment; filename="' . $filename . '"')
            ->withHeader('Cache-Control', 'no-store');
    }

    public static function file(Response $response, string $path, string $contentType, ?string $attachmentName = null): Response
    {
        $stream = fopen($path, 'rb');
        if ($stream === false) throw new RuntimeException("Cannot read $path");
        $response = $response->withBody(new Stream($stream))
            ->withHeader('Content-Type', $contentType)
            ->withHeader('Content-Length', (string) filesize($path))
            ->withHeader('Cache-Control', 'no-store');
        return $attachmentName !== null ? $response->withHeader('Content-Disposition', 'attachment; filename="' . $attachmentName . '"') : $response;
    }

    /** the most bytes one partial answer carries (a video seeks with more requests) */
    public const RANGE_CHUNK = 8388608;

    /**
     * A stored file shown inline, with byte ranges for video seeking: `Range: bytes=a-b` / `a-` / `-n` (one range) →
     * 206 with Content-Range (at most RANGE_CHUNK bytes – a client asks again for the rest); a range past the end → 416;
     * no or an unparsable Range → 200 with the whole file. Never cached (members-only files).
     */
    public static function inline(Response $response, string $path, string $contentType, string $rangeHeader): Response
    {
        $size = (int) filesize($path);
        $response = $response->withHeader('Content-Type', $contentType)->withHeader('Accept-Ranges', 'bytes')
            ->withHeader('Cache-Control', 'private, no-store')->withHeader('X-Content-Type-Options', 'nosniff');
        if (!preg_match('/^bytes=(\d*)-(\d*)$/', trim($rangeHeader), $m) || ($m[1] === '' && $m[2] === '')) {
            $stream = fopen($path, 'rb');
            if ($stream === false) throw new RuntimeException("Cannot read $path");
            return $response->withBody(new Stream($stream))->withHeader('Content-Length', (string) $size);
        }
        if ($m[1] === '') {
            $start = max(0, $size - (int) $m[2]);
            $end = $size - 1;
        } else {
            $start = (int) $m[1];
            $end = $m[2] === '' ? $size - 1 : min((int) $m[2], $size - 1);
        }
        if ($size === 0 || $start >= $size || $end < $start) {
            return $response->withStatus(416)->withHeader('Content-Range', "bytes */$size")->withHeader('Content-Length', '0');
        }
        $end = min($end, $start + self::RANGE_CHUNK - 1);
        $in = fopen($path, 'rb');
        $out = fopen('php://temp', 'w+b');
        if ($in === false || $out === false) throw new RuntimeException("Cannot read $path");
        stream_copy_to_stream($in, $out, $end - $start + 1, $start);
        fclose($in);
        rewind($out);
        return $response->withStatus(206)->withBody(new Stream($out))
            ->withHeader('Content-Range', "bytes $start-$end/$size")->withHeader('Content-Length', (string) ($end - $start + 1));
    }
}
