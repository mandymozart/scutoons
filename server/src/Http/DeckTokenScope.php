<?php
declare(strict_types=1);

namespace Scutoons\Http;

use Scutoons\Domain\ApiError;

/**
 * What a deck token (`Bearer scd_…`, Domain\Decks\Decks, #180) may do: read with its member's rights – nothing else. A
 * deck is read-only for content (agents/research/CYBERDECKS.md §9): every write is refused, and so are the account's
 * own sign-in routes (managing decks and AI connections stays a real sign-in on the account page). The path is matched
 * as the router sees it (decoded, without a trailing slash), like ApiTokenScope.
 */
final class DeckTokenScope
{
    private const READ = ['GET', 'HEAD', 'OPTIONS'];
    /** read routes a deck still may not use */
    private const REFUSED = '#^/api/(me/(tokens|decks)|auth)(/.*)?$#';

    public static function guard(string $method, string $path): void
    {
        $method = strtoupper($method);
        if ($method === 'OPTIONS') return;
        $path = rtrim(rawurldecode($path), '/');
        if (preg_match('#^/api/decks/(request|challenge|token)$#', $path) === 1) return; // the handshake ignores the session
        if (!in_array($method, self::READ, true) || preg_match(self::REFUSED, $path) === 1) {
            throw new ApiError(403, 'not_for_deck', 'A deck only reads, with its member\'s rights – changes are made in the app.');
        }
    }
}
