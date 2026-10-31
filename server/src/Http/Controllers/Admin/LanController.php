<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Admin;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Lan\LanCert;
use Scutoons\Domain\Lan\LanNames;
use Scutoons\Http\Controllers\DevController;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Infrastructure\Config\Settings;

/**
 * The venue relay's names and certificate (#201, docs/API.md "LAN names"; Domain\Lan).
 *
 * Role admin (the deck token with #180):
 *   GET /admin/lan-names          → { status, domain, items: [{ name, host, ip, record_id }] } · 503 not_configured
 *   PUT /admin/lan-names/{name}   { ip } → { status, item: { name, host, ip, change: added|updated|unchanged } }
 *                                 · 422 invalid_name / invalid_ip · 502 kas_error · 503 not_configured
 *   GET /admin/lan-cert           → { status, domain, names, expires, stored_at, fullchain, privkey } · 404 no_certificate
 *       ?format=pem               → the chain and then the key as one PEM text (deck-tls)
 * The workflow lan-cert.yml (no session – the deploy key `X-Migrate-Key: <MIGRATE_KEY>`, like /deploy/migrate):
 *   PUT /deploy/lan-cert          { fullchain, privkey } → { status, names, expires, stored_at } · 422 when it is not
 *                                 the wildcard, expired or the key is not its key · 404 when MIGRATE_KEY is empty
 */
final class LanController
{
    public function __construct(private readonly LanNames $names, private readonly LanCert $cert, private readonly Settings $settings)
    {
    }

    public function names(Request $request, Response $response): Response
    {
        return Json::response($response, 200, ['status' => 'success', 'domain' => LanNames::domain(), 'items' => $this->names->list()]);
    }

    /** @param array{name: string} $args */
    public function setName(Request $request, Response $response, array $args): Response
    {
        $ip = JsonBody::of($request)['ip'] ?? null;
        if (!is_string($ip)) throw new ApiError(422, 'invalid_ip', 'The body is { "ip": "192.168.…" }.');
        return Json::response($response, 200, ['status' => 'success', 'item' => $this->names->set($args['name'], $ip)]);
    }

    public function cert(Request $request, Response $response): Response
    {
        $cert = $this->cert->get();
        if (($request->getQueryParams()['format'] ?? '') === 'pem') {
            return Json::raw($response, 200, rtrim($cert['fullchain']) . "\n" . rtrim($cert['privkey']) . "\n", 'application/x-pem-file')
                ->withHeader('Cache-Control', 'no-store');
        }
        return Json::response($response, 200, ['status' => 'success', 'domain' => LanNames::domain()] + $cert);
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
