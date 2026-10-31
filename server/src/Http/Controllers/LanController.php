<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Lan\LanCert;
use Scutoons\Http\CurrentSession;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Infrastructure\Config\Settings;

/**
 * The venue certificate `*.lan.scutoons.com` (#201, docs/API.md "The venue certificate"; Domain\Lan\LanCert).
 *
 * An approved deck – its own token `Bearer scd_…` (#180), nothing else (owner 2026-10-31: no admin tokens on decks):
 *   GET /decks/lan-cert           → { status, domain, names, expires, stored_at, fullchain, privkey } · 404 no_certificate
 *       ?format=pem               → the chain and then the key as one PEM text (deck-tls)
 *                                 · 401 without a valid deck token (none, pending, revoked, expired) · 403 deck_only
 *                                 for every other sign-in (a session, an AI connection)
 * The workflow lan-cert.yml (no session – the deploy key `X-Migrate-Key: <MIGRATE_KEY>`, like /deploy/migrate):
 *   PUT /deploy/lan-cert          { fullchain, privkey } → { status, names, expires, stored_at } · 422 when it is not
 *                                 the wildcard, expired or the key is not its key · 404 when MIGRATE_KEY is empty
 */
final class LanController
{
    public function __construct(private readonly LanCert $cert, private readonly Settings $settings)
    {
    }

    public function deckCert(Request $request, Response $response): Response
    {
        $session = CurrentSession::of($request);
        if ($session->token() !== null && !$session->isDeckToken()) {
            throw new ApiError(403, 'deck_only', 'Only a paired deck fetches the venue certificate, with its own token.');
        }
        $session->requireUser(); // the deck token: approved, not revoked, not expired (Decks::user)
        $cert = $this->cert->get();
        if (($request->getQueryParams()['format'] ?? '') === 'pem') {
            return Json::raw($response, 200, rtrim($cert['fullchain']) . "\n" . rtrim($cert['privkey']) . "\n", 'application/x-pem-file')
                ->withHeader('Cache-Control', 'no-store');
        }
        return Json::response($response, 200, ['status' => 'success', 'domain' => LanCert::DOMAIN] + $cert)
            ->withHeader('Cache-Control', 'no-store');
    }

    public function storeCert(Request $request, Response $response): Response
    {
        $key = $this->settings->get('MIGRATE_KEY');
        if ($key === '') throw new ApiError(404, 'not_found', 'Not found.');
        $given = $request->getHeaderLine(DevController::KEY_HEADER);
        if ($given === '' || !hash_equals($key, $given)) throw new ApiError(403, 'forbidden', 'Wrong key.');
        $body = JsonBody::of($request);
        $fullchain = $body['fullchain'] ?? null;
        $privkey = $body['privkey'] ?? null;
        if (!is_string($fullchain) || !is_string($privkey)) throw new ApiError(422, 'invalid_body', 'The body is { "fullchain": "<PEM>", "privkey": "<PEM>" }.');
        return Json::response($response, 200, ['status' => 'success'] + $this->cert->store($fullchain, $privkey));
    }
}
