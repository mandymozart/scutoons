<?php
declare(strict_types=1);

namespace Scutoons\Http\Controllers;

use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Scutoons\Domain\ApiError;

/** Any other /api/ path → 404 not_found "Unknown endpoint." (as the old api/index.php answered) */
final class NotFoundController
{
    public function __invoke(Request $request, Response $response): Response
    {
        throw new ApiError(404, 'not_found', 'Unknown endpoint.');
    }
}
