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
 *   GET /api/addresses/<toon> → the document itself (no envelope), `ETag: "<version>"`; `If-None-Match` with that
 *   tag → 304 · 401 not signed in · 403 forbidden (a sign-in without the creator, admin or live role) · 404 unknown_toon
 *   GET /api/addresses?toons=<a>,<b> → several toons' books in one answer (#708, AddressBooks: shared entries once, a
 *   section per toon, the first the home), the same ETag / 304 · 400 invalid_toons (none, more than 8, or not a toon id) ·
 *   404 unknown_toon
 * Read by the team – a sign-in with the creator, admin or live role (an AI connection acts with its member's roles) – and by
 * an approved deck's own token (the #470 precedent: a paired deck reads what the stage needs).
 */
final class AddressBookController
{
    public function __construct(private readonly AddressBook $book, private readonly Roles $roles)
    {
    }

    private const MAX_TOONS = 8;

    /** @param array{toon: string} $args */
    public function get(Request $request, Response $response, array $args): Response
    {
        $this->allowed($request);
        $doc = $this->book->document($args['toon']) ?? throw new ApiError(404, 'unknown_toon', 'No such toon.');
        return $this->answer($request, $response, $doc);
    }

    public function many(Request $request, Response $response): Response
    {
        $this->allowed($request);
        $asked = $request->getQueryParams()['toons'] ?? '';
        $toons = array_values(array_unique(array_filter(array_map('trim', explode(',', is_string($asked) ? $asked : '')), fn(string $t) => $t !== '')));
        if ($toons === [] || count($toons) > self::MAX_TOONS) throw new ApiError(400, 'invalid_toons', 'Name 1 to ' . self::MAX_TOONS . ' toons: ?toons=a,b');
        foreach ($toons as $t) {
            if (preg_match('/^[a-z0-9][a-z0-9-]{0,63}$/', $t) !== 1) throw new ApiError(400, 'invalid_toons', "Not a toon id: $t");
        }
        $doc = $this->book->books($toons);
        if (is_string($doc)) throw new ApiError(404, 'unknown_toon', "No such toon: $doc");
        return $this->answer($request, $response, $doc);
    }

    private function allowed(Request $request): void
    {
        $session = CurrentSession::of($request);
        $user = $session->requireUser(); // a deck token: approved, not revoked, not expired
        if (!$session->isDeckToken() && !$this->roles->canEditContent($user) && !$this->roles->has($user, Roles::LIVE)) {
            throw new ApiError(403, 'forbidden', 'The address book is for the team (creator, admin, live) and paired decks.');
        }
    }

    /** @param array{version: string, json: string} $doc */
    private function answer(Request $request, Response $response, array $doc): Response
    {
        $etag = '"' . $doc['version'] . '"';
        $response = $response->withHeader('ETag', $etag)->withHeader('Access-Control-Expose-Headers', 'ETag')->withHeader('Cache-Control', 'private, no-cache');
        $asked = array_map('trim', explode(',', $request->getHeaderLine('If-None-Match')));
        if (in_array($etag, $asked, true) || in_array("W/$etag", $asked, true)) return $response->withStatus(304);
        $response->getBody()->write($doc['json']);
        return $response->withStatus(200)->withHeader('Content-Type', 'application/json; charset=utf-8');
    }
}
