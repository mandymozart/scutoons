<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Library\ExtendedLibrary;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\CurrentSession;

/**
 * The extended Library (#864, Domain\Library\ExtendedLibrary; docs/API.md "The extended Library"; the shapes:
 * server/schema/api/library.schema.json):
 *   GET /api/library[?toon=&place=&type=&subtype=&kind=&from=&locale=&cursor=&limit=]   a place's page of things and files
 *   GET /api/library?at=<address>[&locale=]                                              a thing (or a file) stepped into
 * `ETag: "<version>"`, `If-None-Match` with it → 304 and no body. Read by a member's session (cookie or Bearer, an AI
 * connection with its member's roles) or a deck's token (with its paired member's roles) – content editors and the `live`
 * role; what each reads is as reading it today (ExtendedLibrary): files, Kanon texts, people and generators content
 * editors, shows `live`.
 */
final class ExtendedLibraryController
{
    public function __construct(private readonly ExtendedLibrary $library, private readonly Roles $roles)
    {
    }

    public function get(Request $request, Response $response): Response
    {
        $user = CurrentSession::of($request)->requireUser();
        $editor = $this->roles->canEditContent($user);
        $live = $this->roles->has($user, Roles::LIVE);
        if (!$editor && !$live) throw new ApiError(403, 'forbidden', 'The Library is for content editors and the live role.', ['role' => Roles::CREATOR]);
        $query = ExtendedLibrary::query($request->getQueryParams());
        ['version' => $version, 'body' => $body] = $this->library->answer($query, $editor, $live);
        $etag = '"' . $version . '"';
        $response = $response->withHeader('ETag', $etag)->withHeader('Access-Control-Expose-Headers', 'ETag')->withHeader('Cache-Control', 'private, no-cache');
        $asked = array_map('trim', explode(',', $request->getHeaderLine('If-None-Match')));
        if (in_array($etag, $asked, true) || in_array("W/$etag", $asked, true)) return $response->withStatus(304);
        $response->getBody()->write($body());
        return $response->withStatus(200)->withHeader('Content-Type', 'application/json; charset=utf-8');
    }
}
