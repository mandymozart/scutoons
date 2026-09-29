<?php
declare(strict_types=1);

namespace Scutoons\Http;

use Psr\Http\Message\ResponseInterface as Response;
use Scutoons\Support\Json as JsonEncoding;

/** JSON responses of the API: unescaped slashes/unicode, `Cache-Control: no-store` unless told otherwise */
final class Json
{
    /** @param array<string, string> $headers */
    public static function response(Response $response, int $status, mixed $data, array $headers = [], int $flags = JsonEncoding::FLAGS): Response
    {
        $response->getBody()->write(JsonEncoding::encode($data, $flags));
        $response = $response->withStatus($status)
            ->withHeader('Content-Type', 'application/json; charset=utf-8')
            ->withHeader('Cache-Control', 'no-store');
        foreach ($headers as $name => $value) $response = $response->withHeader($name, $value);
        return $response;
    }

    /** Raw bytes with a content type (files, CSV, stored JSON) */
    public static function raw(Response $response, int $status, string $body, string $contentType): Response
    {
        $response->getBody()->write($body);
        return $response->withStatus($status)->withHeader('Content-Type', $contentType);
    }
}
