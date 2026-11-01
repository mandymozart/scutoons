<?php
declare(strict_types=1);

namespace Scutoons\Http;

use Scutoons\Domain\ApiError;

/**
 * What a deck token (`Bearer scd_…`, Domain\Decks\Decks, #180) may do: read with its member's rights – and write only
 * what its member granted it at approval (#270, `Decks::SCOPES`). A deck is read-only for content
 * (agents/research/CYBERDECKS.md §9): every other write is refused, and so are the account's own sign-in routes
 * (managing decks and AI connections stays a real sign-in on the account page). The path is matched as the router sees
 * it (decoded, without a trailing slash), like ApiTokenScope.
 *
 * Scopes → writes: `rigs` = POST / PUT / DELETE under /api/rigs (the member's own presets and rig variants – "save as
 * preset" from the deck's knobs, agents/research/INSTRUMENTS.md §4–6).
 */
final class DeckTokenScope
{
    private const READ = ['GET', 'HEAD', 'OPTIONS'];
    /** read routes a deck still may not use */
    private const REFUSED = '#^/api/(me/(tokens|decks)|auth)(/.*)?$#';
    /** scope → the write routes it opens */
    private const WRITES = ['rigs' => '#^/api/rigs(/.*)?$#'];

    /** @param list<string> $scopes the token's write scopes (Decks::scopes) */
    public static function guard(string $method, string $path, array $scopes = []): void
    {
        $method = strtoupper($method);
        if ($method === 'OPTIONS') return;
        $path = rtrim(rawurldecode($path), '/');
        if (preg_match('#^/api/decks/(request|challenge|token)$#', $path) === 1) return; // the handshake ignores the session
        if (in_array($method, self::READ, true)) {
            if (preg_match(self::REFUSED, $path) === 1) throw self::refused();
            return;
        }
        foreach ($scopes as $scope) {
            if (isset(self::WRITES[$scope]) && preg_match(self::WRITES[$scope], $path) === 1) return;
        }
        throw self::refused();
    }

    private static function refused(): ApiError
    {
        return new ApiError(403, 'not_for_deck', 'A deck only reads, with its member\'s rights – changes are made in the app (or need a scope granted at pairing).');
    }
}
