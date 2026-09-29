<?php
declare(strict_types=1);

namespace Scutoons\Http\Middleware;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Psr\Http\Server\MiddlewareInterface;
use Psr\Http\Server\RequestHandlerInterface as Handler;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Http\CurrentSession;

/**
 * `Authorization: Bearer <session_token>` → a CurrentSession attribute. The session is looked up lazily (only
 * endpoints that ask for the user touch the database). Apache/FastCGI may hide the header: .htaccess copies it into
 * HTTP_AUTHORIZATION / REDIRECT_HTTP_AUTHORIZATION, read here as a fallback.
 */
final class Session implements MiddlewareInterface
{
    public function __construct(private readonly Sessions $sessions)
    {
    }

    public function process(Request $request, Handler $handler): Response
    {
        $header = $request->getHeaderLine('Authorization');
        if ($header === '') {
            $server = $request->getServerParams();
            $header = (string) ($server['HTTP_AUTHORIZATION'] ?? $server['REDIRECT_HTTP_AUTHORIZATION'] ?? '');
        }
        $token = Sessions::tokenFromHeader($header);
        return $handler->handle($request->withAttribute(CurrentSession::ATTRIBUTE, new CurrentSession($this->sessions, $token)));
    }
}
