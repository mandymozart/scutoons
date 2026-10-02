<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Certificates\CertificateVerifier;
use Scutoons\Http\Json;
use Scutoons\Http\Middleware\JsonBody;

/**
 * Public certificate verification (the qr app's /verify.html; open to every origin; per-IP lookup limit):
 *   GET  /api/certificates/{code}     the certificate we issued for a code (any case); also ?code= (e.g. a pasted
 *                                     QR / verify URL, which the path cannot carry)
 *   POST /api/certificates/verify     an uploaded SIGNATURE-<code>.json, checked against our key and our records
 * → 200 { status, valid, reason, source, code, registered, revoked, certificate } (docs: agents/ITEMS.md
 * "Certificates") · 400 invalid_code / invalid_signature_file / invalid_json · 429 · 503 keys_missing
 */
final class CertificatesController
{
    public function __construct(private readonly CertificateVerifier $verifier)
    {
    }

    /** @param array{code?: string} $args */
    public function byCode(Request $request, Response $response, array $args): Response
    {
        $code = $args['code'] ?? $request->getQueryParams()['code'] ?? $request->getQueryParams()['c'] ?? null;
        return Json::response($response, 200, $this->verifier->byCode($code));
    }

    public function byFile(Request $request, Response $response): Response
    {
        return Json::response($response, 200, $this->verifier->byFile(JsonBody::of($request)));
    }
}
