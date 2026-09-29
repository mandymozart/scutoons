<?php
declare(strict_types=1);

/**
 * Front controller: every request that is not a static file in public/ (verify.html, assets/, the generate/ SPA files)
 * runs through the Slim app (src/Kernel.php, routes in config/routes.php).
 */
require __DIR__ . '/../vendor/autoload.php';

Scutoons\Kernel::app()->run();
