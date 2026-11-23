<?php
declare(strict_types=1);

namespace Scutoons\Http;

use Scutoons\Domain\ApiError;

/**
 * What the native editor's token (`Bearer sce_…`, Domain\Decks\Decks, kind editor) may do: read with its member's
 * rights and save drafts – a new draft version of an entity, a show or a deck scene – and its stills (/api/thumbs). Publishing stays in the browser
 * (decisions/deck.md 2026-11-09): every other write is refused, and so are the account's own sign-in routes, like a
 * deck's (DeckTokenScope). The path is matched as the router sees it (decoded, without a trailing slash).
 */
final class EditorTokenScope
{
    private const READ = ['GET', 'HEAD'];
    /** read routes the editor still may not use */
    private const REFUSED = '#^/api/(me/(tokens|decks)|auth)(/.*)?$#';
    /** the draft saves */
    private const DRAFTS = '#^/api/(entities|shows|scenes)/[^/]+$#';

    public static function guard(string $method, string $path): void
    {
        $method = strtoupper($method);
        if ($method === 'OPTIONS') return;
        $path = rtrim(rawurldecode($path), '/');
        if (preg_match('#^/api/decks/(request|challenge|token)$#', $path) === 1) return;
        if (in_array($method, self::READ, true)) {
            if (preg_match(self::REFUSED, $path) === 1) throw self::refused();
            return;
        }
        if ($method === 'PUT' && preg_match(self::DRAFTS, $path) === 1) return;
        if ($method === 'PUT' && $path === '/api/thumbs') return; // the editor's stills
        throw self::refused();
    }

    private static function refused(): ApiError
    {
        return new ApiError(403, 'not_for_editor', 'The editor reads and saves drafts – publishing and the account are in the browser.');
    }
}
