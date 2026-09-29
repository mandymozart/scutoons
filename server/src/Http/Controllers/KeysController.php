<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Infrastructure\Signing\KeyStore;

/**
 * PUBLIC URL – must keep working unchanged: GET /api/keys/keys.php → the ML-DSA-65 PUBLIC key of the certificate
 * signatures { success, publicKey, algorithm, generated } (404 { success: false, error } without keys).
 * Never the private key: the admin API (/api/admin/keys, role admin) and the legacy generate/keys.php have that.
 */
final class KeysController
{
    public function __construct(private readonly KeyStore $keys)
    {
    }

    public function __invoke(Request $request, Response $response): Response
    {
        $keys = $this->keys->keypair();
        $response = $response->withHeader('Content-Type', 'application/json')->withHeader('Cache-Control', 'no-store');
        if ($keys === null || !isset($keys['publicKey'])) {
            $response->getBody()->write((string) json_encode(['success' => false, 'error' => 'Public key not available']));
            return $response->withStatus(404);
        }
        $response->getBody()->write((string) json_encode([
            'success' => true,
            'publicKey' => $keys['publicKey'],
            'algorithm' => $keys['algorithm'] ?? 'ML-DSA-65',
            'generated' => $keys['generated'] ?? null,
        ]));
        return $response;
    }
}
