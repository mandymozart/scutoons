<?php
declare(strict_types=1);

namespace Scutoons\Http\Middleware;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Psr\Http\Server\MiddlewareInterface;
use Psr\Http\Server\RequestHandlerInterface as Handler;
use Scutoons\Domain\Auth\ApiTokens;
use Scutoons\Domain\Auth\Sessions;
use Scutoons\Domain\Decks\Decks;
use Scutoons\Http\AllowedOrigins;
use Scutoons\Http\ApiTokenScope;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\DeckTokenScope;
use Scutoons\Http\SessionCookie;
use Scutoons\Infrastructure\Config\Settings;

/**
 * `Authorization: Bearer <session_token>` or the single sign-on cookie `scutoons_sid` → a CurrentSession attribute.
 * The Bearer header wins (old clients keep working unchanged). The session is looked up lazily (only endpoints that ask
 * for the user touch the database). Apache/FastCGI may hide the header: .htaccess copies it into HTTP_AUTHORIZATION /
 * REDIRECT_HTTP_AUTHORIZATION, read here as a fallback.
 *
 * CSRF: a cookie on POST/PUT/DELETE/PATCH counts only with an `Origin` from ALLOWED_ORIGINS (CurrentSession).
 *
 * A personal access token (`Bearer sct_…`, an AI connection) is refused on what only the member does in the app
 * (ApiTokenScope: publish, send, apply, manage tokens, admin changes) – 403 not_for_ai. A deck token (`Bearer scd_…`)
 * only reads (DeckTokenScope) – 403 not_for_deck – unless its member granted it a write scope at approval (#270).
 */
final class Session implements MiddlewareInterface
{
    private const SAFE_METHODS = ['GET', 'HEAD', 'OPTIONS'];

    public function __construct(private readonly Sessions $sessions, private readonly Settings $settings, private readonly ApiTokens $apiTokens, private readonly Decks $decks)
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
        $session = new CurrentSession($this->sessions, $token, $via, $csrfBlocked, $this->apiTokens, $this->decks);
        if ($session->isApiToken()) ApiTokenScope::guard($request->getMethod(), $request->getUri()->getPath());
        if ($session->isDeckToken()) DeckTokenScope::guard($request->getMethod(), $request->getUri()->getPath(), $this->decks->scopes((string) $token));
        return $handler->handle($request->withAttribute(CurrentSession::ATTRIBUTE, $session));
    }
}
