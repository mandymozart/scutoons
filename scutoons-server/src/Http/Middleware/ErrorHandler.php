<?php
declare(strict_types=1);

namespace Scutoons\Http\Middleware;

use Psr\Http\Message\ResponseFactoryInterface;
use Psr\Http\Message\ResponseInterface as Response;
use Psr\Http\Message\ServerRequestInterface as Request;
use Psr\Http\Server\MiddlewareInterface;
use Psr\Http\Server\RequestHandlerInterface as Handler;
use Scutoons\Domain\ApiError;
use Scutoons\Http\Json;
use Slim\Exception\HttpMethodNotAllowedException;
use Slim\Exception\HttpNotFoundException;
use Throwable;

/**
 * Every error as `{ status: "error", error: "<code>", message, …extra }` (agents/ITEMS.md):
 * ApiError → its status/code; no route (or a method the route does not take) → 404 not_found "Unknown endpoint.";
 * anything else → logged, 500 server_error (no details leak).
 */
final class ErrorHandler implements MiddlewareInterface
{
    public function __construct(private readonly ResponseFactoryInterface $responses)
    {
    }

    public function process(Request $request, Handler $handler): Response
    {
        try {
            return $handler->handle($request);
        } catch (ApiError $error) {
            return $this->render($error);
        } catch (HttpNotFoundException | HttpMethodNotAllowedException) {
            return $this->render(new ApiError(404, 'not_found', 'Unknown endpoint.'));
        } catch (Throwable $error) {
            error_log('[qr-api] ' . $error::class . ': ' . $error->getMessage() . ' in ' . $error->getFile() . ':' . $error->getLine());
            return $this->render(new ApiError(500, 'server_error', 'Something went wrong on the server.'));
        }
    }

    public function render(ApiError $error): Response
    {
        return Json::response($this->responses->createResponse(), $error->status, $error->toArray());
    }
}
