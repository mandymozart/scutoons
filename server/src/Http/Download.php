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
}
