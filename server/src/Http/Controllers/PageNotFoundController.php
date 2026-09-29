<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;

/**
 * Any path outside /api/ and the legacy generate/ → 404 "Not found" (plain text). This server is the API only; the
 * public pages (QR landing /?c=, /verify.html) are the qr app (qr/, qr.scutoons.com).
 */
final class PageNotFoundController
{
    public function __invoke(Request $request, Response $response): Response
    {
        $response->getBody()->write('Not found');
        return $response->withStatus(404)->withHeader('Content-Type', 'text/plain; charset=UTF-8');
    }
}
