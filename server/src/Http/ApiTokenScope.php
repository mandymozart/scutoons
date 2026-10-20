<?php
declare(strict_types=1);

namespace Scutoons\Http;

use Scutoons\Domain\ApiError;

/**
 * What a personal access token (an AI connection, agents/MCP.md "Draft → link") may not do, whatever the member's
 * roles: the steps that reach readers or people outside, or change things wholesale, stay the member's click in the
 * app – publishing (one document or many, #100), replacing a file in place (readers see the new bytes at once), sending
 * a mail (an Embassy mail), applying a proposal, deciding a review, admin changes (the admin's AI only reads and
 * proposes), and the account's own sign-in (cookies, sign-out of every device, managing tokens). Everything else is the
 * member's rights as usual – invites and gifts too: what the admin's campaigns and the daily limits allow the member
 * (owner 2026-10-21). The path is matched as the router sees it: decoded like Slim's RouteResolver (`%61dmin` routes to
 * `admin`), without a trailing slash.
 */
final class ApiTokenScope
{
    /** [method, path pattern] the token is refused on */
    private const REFUSED = [
        ['POST', '#^/api/auth/(session|logout)$#'],
        ['*', '#^/api/me/tokens(/.*)?$#'],
        ['POST', '#^/api/entities(/[^/]+)?/(publish|unpublish)$#'],
        ['PUT', '#^/api/assets/replace$#'],
        ['POST', '#^/api/reviews/(publish|return)$#'],
        ['POST', '#^/api/embassy/orgs/[^/]+/mail$#'],
        ['DELETE', '#^/api/embassy/orgs/[^/]+$#'],
        ['POST', '#^/api/embassy/import$#'],
    ];

    public static function guard(string $method, string $path): void
    {
        $method = strtoupper($method);
        $path = rtrim(rawurldecode($path), '/');
        if ($method === 'OPTIONS') return;
        // admin: reading, and saving a proposal the admin applies in the app
        $refused = str_starts_with($path, '/api/admin')
            && !in_array($method, ['GET', 'HEAD'], true)
            && !($method === 'POST' && $path === '/api/admin/proposals');
        foreach (self::REFUSED as [$m, $pattern]) {
            if (($m === '*' || $m === $method) && preg_match($pattern, $path) === 1) $refused = true;
        }
        if ($refused) {
            throw new ApiError(403, 'not_for_ai', 'An AI connection cannot do this – the member does it in the app (publish, send, apply, admin changes, sign-in).');
        }
    }
}
