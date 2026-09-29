<?php
declare(strict_types=1);

/**
 * Front controller: every request that is not a static file in public/ (verify.html, assets/, generate/ SPA files)
 * runs through the Slim app (src/Kernel.php, routes in config/routes.php).
 */
require __DIR__ . '/../vendor/autoload.php';

use Scutoons\Http\Controllers\LegacyBridge;
use Scutoons\Kernel;
use Slim\Factory\ServerRequestCreatorFactory;
use Slim\ResponseEmitter;

$app = Kernel::app();
$request = ServerRequestCreatorFactory::create()->createServerRequestFromGlobals();
$response = $app->handle($request);

// TEMPORARY (refactor step 1): a module that has not moved yet runs its old script at global scope
if ($response->hasHeader(LegacyBridge::HEADER)) {
    $__legacyScript = $response->getHeaderLine(LegacyBridge::HEADER);
    unset($app, $request, $response);
    chdir(dirname($__legacyScript));
    require $__legacyScript;
    exit;
}

(new ResponseEmitter())->emit($response);
