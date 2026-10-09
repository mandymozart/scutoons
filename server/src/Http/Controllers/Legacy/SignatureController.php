<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers\Legacy;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\Admin\CodeAdmin;
use Scutoons\Domain\ApiError;
use Scutoons\Http\Json;
use Scutoons\Infrastructure\Storage\FileStore;

/**
 * LEGACY URLs, PUBLIC – the two endpoints of the retired generate/ SPA that the verify page (qr app, proxied) and
 * printed certificate links still use: generate/get-signature.php and generate/files/signatures/SIGNATURE-<code>.json.
 * URL and answer shape stay unchanged (`{ success, … }`). The rest of generate/ (the static-password admin SPA and its
 * endpoints) was removed 2026-10-09 – the admin app (/api/admin/*, role admin) does all of it.
 */
final class SignatureController
{
    public function __construct(private readonly CodeAdmin $codes, private readonly FileStore $files)
    {
    }

    /** GET get-signature.php?code= → { success, signature, fingerprint, qr_data, redirect_to } · 404 when unknown or unsigned */
    public function signature(Request $request, Response $response): Response
    {
        if ($request->getMethod() !== 'GET') return $this->json($response, 405, ['success' => false, 'message' => 'Method not allowed']);
        $code = $request->getQueryParams()['code'] ?? '';
        if (!is_string($code) || $code === '') return $this->json($response, 200, ['success' => false, 'message' => 'Missing code parameter']);
        try {
            return $this->json($response, 200, ['success' => true] + $this->codes->signature($code));
        } catch (ApiError $error) {
            return $this->json($response, $error->status === 500 ? 200 : $error->status, ['success' => false, 'message' => $error->getMessage()]);
        }
    }

    /**
     * The signature file of a code (verify page fallback: generate/files/signatures/SIGNATURE-<code>.json)
     * @param array{name: string} $args
     */
    public function signatureFile(Request $request, Response $response, array $args): Response
    {
        $content = $this->files->read('signatures', $args['name']);
        if ($content === null) return Json::raw($response, 404, 'Not found', 'text/plain; charset=UTF-8');
        return Json::raw($response, 200, $content, 'application/json');
    }

    /** @param array<string, mixed> $data */
    private function json(Response $response, int $status, array $data): Response
    {
        $response->getBody()->write((string) json_encode($data));
        return $response->withStatus($status)->withHeader('Content-Type', 'application/json');
    }
}
