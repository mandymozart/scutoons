<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Infrastructure\Config\Settings;

/**
 * TEMPORARY (refactor step 1 only): paths whose module has not moved to Slim yet run the old plain-PHP scripts,
 * exactly as the old router.php did. Deleted with the last old script.
 */
final class LegacyBridge
{
    public function __construct(private readonly Settings $settings)
    {
    }

    public function __invoke(Request $request, Response $response): Response
    {
        $root = $this->settings->root();
        $path = rawurldecode($request->getUri()->getPath());
        if (preg_match('#(^|/)(\.|db/|logs/|scripts/|tests/|api/lib/)|keypair\.json$|permissions\.json$|\.htpasswd$|\.sqlite|\.log$#', $path) || str_contains($path, '..')) {
            return $this->notFound($response);
        }
        if (str_starts_with($path, '/api/keys/') && $path !== '/api/keys/keys.php') return $this->notFound($response);

        $file = $root . $path;
        if (str_starts_with($path, '/api/') || $path === '/api') {
            if (is_file($file) && str_ends_with($file, '.php')) return $this->run($file);
            if (is_dir($file) && is_file(rtrim($file, '/') . '/index.php')) return $this->run(rtrim($file, '/') . '/index.php');
            return $this->run($root . '/api/index.php');
        }
        if (str_starts_with($path, '/generate/files/') && is_file($file)) {
            $response->getBody()->write((string) file_get_contents($file));
            $types = ['json' => 'application/json', 'svg' => 'image/svg+xml', 'pdf' => 'application/pdf', 'zip' => 'application/zip'];
            return $response->withHeader('Content-Type', $types[pathinfo($file, PATHINFO_EXTENSION)] ?? 'application/octet-stream');
        }
        if (preg_match('#^/generate/([a-z-]+\.php)$#', $path, $m) && is_file("$root/generate/{$m[1]}")) {
            return $this->run("$root/generate/{$m[1]}");
        }
        return $this->run($root . '/index.php');
    }

    /** public/index.php runs the script at global scope (the old scripts use globals) instead of emitting this */
    private function run(string $script): Response
    {
        return (new \Slim\Psr7\Response())->withHeader(self::HEADER, $script);
    }

    public const HEADER = 'X-Legacy-Script';

    private function notFound(Response $response): Response
    {
        $response->getBody()->write('Not found');
        return $response->withStatus(404);
    }
}
