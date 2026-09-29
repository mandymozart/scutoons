<?php
declare(strict_types=1);

namespace Scutoons\Http\Middleware;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Psr\Http\Server\MiddlewareInterface;
use Psr\Http\Server\RequestHandlerInterface as Handler;
use Scutoons\Domain\ApiError;
use stdClass;

/**
 * Reads the request body once and keeps it as an attribute. Parsing is lazy on purpose: an endpoint that does not
 * read the body never fails on it (as before). JsonBody::of() → the body as an array ({} / empty → []), else
 * 400 invalid_json; JsonBody::object() → decoded with objects (keeps `{}`), for stores of client JSON.
 */
final class JsonBody implements MiddlewareInterface
{
    public const ATTRIBUTE = 'json_body_raw';

    public function process(Request $request, Handler $handler): Response
    {
        return $handler->handle($request->withAttribute(self::ATTRIBUTE, (string) $request->getBody()));
    }

    public static function raw(Request $request): string
    {
        $raw = $request->getAttribute(self::ATTRIBUTE);
        return is_string($raw) ? $raw : (string) $request->getBody();
    }

    /** @return array<mixed> */
    public static function of(Request $request): array
    {
        $raw = self::raw($request);
        if (trim($raw) === '') return [];
        $data = json_decode($raw, true);
        if (!is_array($data)) throw new ApiError(400, 'invalid_json', 'The request body is not a JSON object.');
        return $data;
    }

    /** The body decoded with objects, or null when it is not a JSON object */
    public static function object(Request $request): ?stdClass
    {
        $body = json_decode(self::raw($request));
        return $body instanceof stdClass ? $body : null;
    }
}
