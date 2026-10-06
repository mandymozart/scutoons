<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Library\Answer;
use Scutoons\Domain\Library\Library;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Middleware\Cors;

/**
 * The reader's entity endpoints (agents/DATA.md §3, docs/API.md "Library"), public GETs:
 *   GET /api/library/toons[?locale=]                     ToonIndex[] (with a locale: without the toons closed in it)
 *   GET /api/library/toons/{toon}?locale=                { entry, toon, seasons, locations, routes }
 *   GET /api/library/toons/{toon}/seasons/{n}?locale=    SeasonOverview
 *   GET /api/library/toons/{toon}/chapters/{n}?locale=   Chapter with its scenes inlined
 *   GET /api/library/toons/{toon}/cast[?chapter=]        { registry, cast }
 *   GET /api/library/toons/{toon}/credits                { credits, people, roles }
 *   GET /api/library/characters                          CharacterRegistry (the whole registry)
 *   GET /api/library/map?locale=&toon=                  the world map for this reader (agents/MAP.md)
 *   GET /api/events                                      Event[]
 * `?draft=1` (role creator): the newest versions, no status gates, not cached. Public answers: `Cache-Control: public,
 * max-age=60`, ETag (304 on If-None-Match), `Content-Language` / `X-Content-Fallback` for localized answers, any
 * origin may read them (`*` like /api/products/).
 */
final class LibraryController
{
    private const EXPOSE = 'ETag, Content-Language, X-Content-Fallback, X-Content-Missing';

    public function __construct(private readonly Library $library, private readonly Roles $roles, private readonly Cors $cors)
    {
    }

    public function toons(Request $request, Response $response): Response
    {
        return $this->send($request, $response, $this->library->toons($this->draft($request), $this->locale($request)));
    }

    /** @param array{toon: string} $args */
    public function toon(Request $request, Response $response, array $args): Response
    {
        $locale = $this->locale($request);
        return $this->send($request, $response, $this->library->toon($args['toon'], $locale, $this->draft($request)));
    }

    /** @param array{toon: string, number: string} $args */
    public function season(Request $request, Response $response, array $args): Response
    {
        $locale = $this->locale($request);
        return $this->send($request, $response, $this->library->season($args['toon'], (int) $args['number'], $locale, $this->draft($request)));
    }

    /** @param array{toon: string, number: string} $args */
    public function chapter(Request $request, Response $response, array $args): Response
    {
        $locale = $this->locale($request);
        return $this->send($request, $response, $this->library->chapter($args['toon'], (int) $args['number'], $locale, $this->draft($request)));
    }

    /** @param array{toon: string} $args */
    public function cast(Request $request, Response $response, array $args): Response
    {
        $chapter = $request->getQueryParams()['chapter'] ?? null;
        if ($chapter !== null && (!is_string($chapter) || !preg_match('#^[0-9]{1,6}$#', $chapter))) {
            throw new ApiError(400, 'invalid_chapter', 'chapter must be a chapter number.');
        }
        return $this->send($request, $response, $this->library->cast($args['toon'], $chapter === null ? null : (int) $chapter, $this->draft($request)));
    }

    /** @param array{toon: string} $args */
    public function credits(Request $request, Response $response, array $args): Response
    {
        return $this->send($request, $response, $this->library->credits($args['toon'], $this->draft($request)));
    }

    public function characters(Request $request, Response $response): Response
    {
        return $this->send($request, $response, $this->library->characters($this->draft($request)));
    }

    /** GET /api/library/map?locale=&toon= (agents/MAP.md §2.3) */
    public function map(Request $request, Response $response): Response
    {
        $toon = $request->getQueryParams()['toon'] ?? null;
        if ($toon !== null && !is_string($toon)) throw new ApiError(400, 'invalid_toon', 'Invalid toon id.');
        return $this->send($request, $response, $this->library->map($this->locale($request), $toon === '' ? null : $toon, $this->draft($request)));
    }

    public function events(Request $request, Response $response): Response
    {
        return $this->send($request, $response, $this->library->events($this->draft($request)));
    }

    private function locale(Request $request): ?string
    {
        $locale = $request->getQueryParams()['locale'] ?? null;
        if ($locale === null || $locale === '') return null;
        if (!is_string($locale) || !preg_match(Library::LOCALE_PATTERN, $locale)) {
            throw new ApiError(400, 'invalid_locale', 'locale must be a two-letter language code.');
        }
        return $locale;
    }

    /** `?draft=1` = creator preview (401 without a session, 403 without the role) */
    private function draft(Request $request): bool
    {
        if (($request->getQueryParams()['draft'] ?? '') !== '1') return false;
        $this->roles->requireContentEditor(CurrentSession::of($request)->user());
        return true;
    }

    private function send(Request $request, Response $response, Answer $answer): Response
    {
        $origin = rtrim($request->getHeaderLine('Origin'), '/');
        $response = $response
            ->withHeader('Content-Type', 'application/json; charset=utf-8')
            ->withHeader('Cache-Control', $answer->draft ? 'private, no-store' : 'public, max-age=60')
            ->withHeader('ETag', $answer->etag)
            ->withHeader('Access-Control-Allow-Origin', $origin !== '' && $this->cors->isAllowed($origin) ? $origin : '*')
            ->withHeader('Access-Control-Expose-Headers', self::EXPOSE);
        if ($answer->locale !== null) $response = $response->withHeader('Content-Language', $answer->locale);
        if ($answer->fallback) $response = $response->withHeader('X-Content-Fallback', '1');
        if ($answer->missing !== []) $response = $response->withHeader('X-Content-Missing', implode(', ', $answer->missing));
        if (!$answer->draft && trim($request->getHeaderLine('If-None-Match')) === $answer->etag) return $response->withStatus(304);
        $response->getBody()->write($answer->body);
        return $response->withStatus(200);
    }
}
