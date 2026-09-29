<?php
declare(strict_types=1);

namespace Scutoons\Http\Middleware;

use Psr\Http\Message\ResponseFactoryInterface;
use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Psr\Http\Server\MiddlewareInterface;
use Psr\Http\Server\RequestHandlerInterface as Handler;
use Scutoons\Infrastructure\Config\Settings;

/**
 * LEGACY – the static admin token of the generate/ SPA: `X-Generate-Token` header (or ?token=) must equal
 * HMAC-SHA256("generate-access", GENERATE_PASSWORD), else 401 {"error":"Unauthorized"}. Only for the /generate/*.php
 * endpoints; removed together with generate/ once the admin app (role admin, /api/admin/*) has parity.
 */
final class GenerateToken implements MiddlewareInterface
{
    public function __construct(private readonly Settings $settings, private readonly ResponseFactoryInterface $responses)
    {
    }

    public static function expected(string $password): string
    {
        return hash_hmac('sha256', 'generate-access', $password);
    }

    public function process(Request $request, Handler $handler): Response
    {
        $token = $request->getHeaderLine('X-Generate-Token');
        if ($token === '') {
            $query = $request->getQueryParams()['token'] ?? '';
            $token = is_string($query) ? $query : '';
        }
        $password = $this->settings->get('GENERATE_PASSWORD');
        if ($password === '' || !hash_equals(self::expected($password), $token)) {
            $response = $this->responses->createResponse(401)->withHeader('Content-Type', 'application/json');
            $response->getBody()->write((string) json_encode(['error' => 'Unauthorized']));
            return $response;
        }
        return $handler->handle($request);
    }
}
