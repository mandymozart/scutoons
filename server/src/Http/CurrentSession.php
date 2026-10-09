<?php
declare(strict_types=1);

namespace Scutoons\Http;

use Psr\Http\Message\ServerRequestInterface as Request;
use RuntimeException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Auth\ApiTokens;
use Scutoons\Domain\Auth\Sessions;

/**
 * The session of a request (request attribute, set by the Session middleware): a Bearer token, or – single sign-on – the
 * `scutoons_sid` cookie (SessionCookie). The Bearer header wins when both are sent.
 *
 * CSRF (docs/API.md "Single sign-on"): a cookie on a state-changing request (POST/PUT/DELETE) only counts when the
 * request's Origin is one of ALLOWED_ORIGINS. Otherwise the request is anonymous (`user()` = null) and endpoints that need
 * a user answer `403 csrf_rejected` – public endpoints that only optionally look at the user (e.g. /access/, /validate/)
 * keep working for everybody.
 *
 * A personal access token (`Bearer sct_…`, ApiTokens – "AI connections", agents/MCP.md) is a Bearer like a session: the
 * member's account while it has a member role; what it may not do, ApiTokenScope refuses.
 */
final class CurrentSession
{
    public const ATTRIBUTE = 'session';
    public const VIA_BEARER = 'bearer';
    public const VIA_COOKIE = 'cookie';

    private bool $resolved = false;
    /** @var array<string, mixed>|null */
    private ?array $user = null;

    /**
     * @param string|null $via         VIA_BEARER | VIA_COOKIE | null (no token)
     * @param bool        $csrfBlocked a cookie on an unsafe request from an origin outside ALLOWED_ORIGINS
     */
    public function __construct(
        private readonly Sessions $sessions,
        private readonly ?string $token,
        private readonly ?string $via = null,
        private readonly bool $csrfBlocked = false,
        private readonly ?ApiTokens $apiTokens = null,
    ) {
    }

    public static function of(Request $request): self
    {
        $session = $request->getAttribute(self::ATTRIBUTE);
        if (!$session instanceof self) throw new RuntimeException('Session middleware missing');
        return $session;
    }

    /** The usable session token (null: none, or a cookie the CSRF check refused) */
    public function token(): ?string
    {
        return $this->csrfBlocked ? null : $this->token;
    }

    /** How the token came: VIA_BEARER, VIA_COOKIE or null */
    public function via(): ?string
    {
        return $this->csrfBlocked ? null : $this->via;
    }

    /** The request came with a personal access token (an AI connection), not a session */
    public function isApiToken(): bool
    {
        return $this->via() === self::VIA_BEARER && ApiTokens::isApiToken($this->token);
    }

    /** @return array{id: string, name: string}|null the AI connection's id and name */
    public function apiToken(): ?array
    {
        return $this->isApiToken() && $this->token !== null ? $this->apiTokens?->token($this->token) : null;
    }

    public function csrfBlocked(): bool
    {
        return $this->csrfBlocked;
    }

    /** @return array<string, mixed>|null the signed-in user, or null (no/invalid/expired token, refused cookie) */
    public function user(): ?array
    {
        if (!$this->resolved) {
            $token = $this->token();
            $this->user = $token === null ? null : ($this->isApiToken() ? $this->apiTokens?->user($token) : $this->sessions->user($token));
            $this->resolved = true;
        }
        return $this->user;
    }

    /**
     * The signed-in user, else 401 unauthorized (403 csrf_rejected for a cookie from a foreign origin)
     * @return array<string, mixed>
     */
    public function requireUser(): array
    {
        $this->rejectCsrf();
        return $this->user() ?? throw new ApiError(401, 'unauthorized', 'Not signed in.');
    }

    /** 403 csrf_rejected when the request's cookie was refused by the CSRF check */
    public function rejectCsrf(): void
    {
        if ($this->csrfBlocked) {
            throw new ApiError(403, 'csrf_rejected', 'Signed-in changes are only accepted from the SCUTOONS apps.');
        }
    }
}
