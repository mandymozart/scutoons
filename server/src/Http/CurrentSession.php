<?php
declare(strict_types=1);

namespace Scutoons\Http;

use Psr\Http\Message\ServerRequestInterface as Request;
use RuntimeException;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Auth\Sessions;

/** The Bearer session of a request (request attribute, set by the Session middleware) */
final class CurrentSession
{
    public const ATTRIBUTE = 'session';

    private bool $resolved = false;
    /** @var array<string, mixed>|null */
    private ?array $user = null;

    public function __construct(private readonly Sessions $sessions, private readonly ?string $token)
    {
    }

    public static function of(Request $request): self
    {
        $session = $request->getAttribute(self::ATTRIBUTE);
        if (!$session instanceof self) throw new RuntimeException('Session middleware missing');
        return $session;
    }

    public function token(): ?string
    {
        return $this->token;
    }

    /** @return array<string, mixed>|null the signed-in user, or null (no/invalid/expired token) */
    public function user(): ?array
    {
        if (!$this->resolved) {
            $this->user = $this->token !== null ? $this->sessions->user($this->token) : null;
            $this->resolved = true;
        }
        return $this->user;
    }

    /**
     * The signed-in user, else 401 unauthorized
     * @return array<string, mixed>
     */
    public function requireUser(): array
    {
        return $this->user() ?? throw new ApiError(401, 'unauthorized', 'Not signed in.');
    }
}
