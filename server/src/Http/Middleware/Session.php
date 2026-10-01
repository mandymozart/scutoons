<?php
declare(strict_types=1);

namespace Scutoons\Http\Middleware;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Psr\Http\Server\MiddlewareInterface;
use Psr\Http\Server\RequestHandlerInterface as Handler;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Http\AllowedOrigins;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\SessionCookie;
use Scutoons\Infrastructure\Config\Settings;

/**
 * `Authorization: Bearer <session_token>` or the single sign-on cookie `scutoons_sid` → a CurrentSession attribute.
 * The Bearer header wins (old clients keep working unchanged). The session is looked up lazily (only endpoints that ask
 * for the user touch the database). Apache/FastCGI may hide the header: .htaccess copies it into HTTP_AUTHORIZATION /
 * REDIRECT_HTTP_AUTHORIZATION, read here as a fallback.
 *
 * CSRF: a cookie on POST/PUT/DELETE/PATCH counts only with an `Origin` from ALLOWED_ORIGINS (CurrentSession).
 */
final class Session implements MiddlewareInterface
{
    private const SAFE_METHODS = ['GET', 'HEAD', 'OPTIONS'];

    public function __construct(private readonly Sessions $sessions, private readonly Settings $settings)
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
        $via = $token !== null ? CurrentSession::VIA_BEARER : null;
        $csrfBlocked = false;
        if ($token === null) {
            $token = SessionCookie::token($request);
            if ($token !== null) {
                $via = CurrentSession::VIA_COOKIE;
                $csrfBlocked = !in_array(strtoupper($request->getMethod()), self::SAFE_METHODS, true)
                    && !AllowedOrigins::allows($this->settings, $request->getHeaderLine('Origin'));
            }
        }
        $session = new CurrentSession($this->sessions, $token, $via, $csrfBlocked);
        return $handler->handle($request->withAttribute(CurrentSession::ATTRIBUTE, $session));
    }
}
