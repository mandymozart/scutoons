<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Addresses\AddressBook;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Http\CurrentSession;

/**
 * One toon's address book (#477, Domain\Addresses\AddressBook; the shape: server/schema/api/address-book.schema.json):
 *   GET /api/toons/<toon>/addresses → the document itself (no envelope), `ETag: "<version>"`; `If-None-Match` with that
 *   tag → 304 · 401 not signed in · 403 forbidden (a sign-in without the creator, admin or live role) · 404 unknown_toon
 * Read by the team – a sign-in with the creator, admin or live role (an AI connection acts with its member's roles) – and by
 * an approved deck's own token (the #470 precedent: a paired deck reads what the stage needs).
 */
final class AddressBookController
{
    public function __construct(private readonly AddressBook $book, private readonly Roles $roles)
    {
    }

    /** @param array{toon: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        $session = CurrentSession::of($request);
        $user = $session->requireUser(); // a deck token: approved, not revoked, not expired
        if (!$session->isDeckToken() && !$this->roles->canEditContent($user) && !$this->roles->has($user, Roles::LIVE)) {
            throw new ApiError(403, 'forbidden', 'The address book is for the team (creator, admin, live) and paired decks.');
        }
        $doc = $this->book->document($args['toon']) ?? throw new ApiError(404, 'unknown_toon', 'No such toon.');
        $etag = '"' . $doc['version'] . '"';
        $response = $response->withHeader('ETag', $etag)->withHeader('Cache-Control', 'private, no-cache');
        $asked = array_map('trim', explode(',', $request->getHeaderLine('If-None-Match')));
        if (in_array($etag, $asked, true) || in_array("W/$etag", $asked, true)) return $response->withStatus(304);
        $response->getBody()->write($doc['json']);
        return $response->withStatus(200)->withHeader('Content-Type', 'application/json; charset=utf-8');
    }
}
