<?php
declare(strict_types=1);

namespace Scutoons\Http\Middleware;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Psr\Http\Server\MiddlewareInterface;
use Psr\Http\Server\RequestHandlerInterface as Handler;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\CurrentSession;

/**
 * Route guard: the signed-in user must have the role – no session → 401 unauthorized, without the role → 403
 * forbidden { role }. The user is passed on as the request attribute `user`.
 */
final class RequireRole implements MiddlewareInterface
{
    public function __construct(private readonly Roles $roles, private readonly string $role)
    {
    }

    public function process(Request $request, Handler $handler): Response
    {
        $user = $this->roles->require(CurrentSession::of($request)->user(), $this->role);
        return $handler->handle($request->withAttribute('user', $user));
    }
}
