<?php
declare(strict_types=1);

namespace Scutoons;

use DI\Container;
use DI\ContainerBuilder;
use Scutoons\Http\Middleware\Cors;
use Scutoons\Http\Middleware\DeviceId;
use Scutoons\Http\Middleware\ErrorHandler;
use Scutoons\Http\Middleware\JsonBody;
use Scutoons\Http\Middleware\NormalizePath;
use Scutoons\Http\Middleware\Session;
use Scutoons\Infrastructure\Config\Settings;
use Slim\App;
use Slim\Factory\AppFactory;

/**
 * Builds the container and the Slim app. Used by public/index.php (web), bin/console (CLI) and the tests.
 * No long-running process: one container per request / command (shared hosting).
 */
final class Kernel
{
    public static function container(?Settings $settings = null): Container
    {
        $settings ??= Settings::load();
        self::configureErrorLog($settings);
        $builder = new ContainerBuilder();
        $builder->useAutowiring(true);
        $builder->addDefinitions($settings->root() . '/config/container.php');
        $builder->addDefinitions([Settings::class => $settings]);
        return $builder->build();
    }

    /** @return App<Container> */
    public static function app(?Container $container = null): App
    {
        $container ??= self::container();
        AppFactory::setContainer($container);
        /** @var App<Container> $app */
        $app = AppFactory::create();
        $settings = $container->get(Settings::class);

        // Middleware runs outside-in in the reverse order of adding: NormalizePath → Cors → ErrorHandler → routing →
        // DeviceId → JsonBody → Session → route middleware (RequireRole, RateLimit) → controller.
        // No host routing: this is the API only; the public pages (qr.scutoons.com) are the qr app, which proxies here.
        $app->add(Session::class);
        $app->add(JsonBody::class);
        $app->add(DeviceId::class);
        $app->addRoutingMiddleware();
        $app->add(ErrorHandler::class);
        $app->add(Cors::class);
        $app->add(NormalizePath::class);

        (require $settings->root() . '/config/routes.php')($app);
        return $app;
    }

    /** PHP errors go to LOG_PATH (logs/ is never served), never to the page */
    private static function configureErrorLog(Settings $settings): void
    {
        ini_set('display_errors', '0');
        $logPath = $settings->path('LOG_PATH');
        if ($logPath === '') return;
        $dir = dirname($logPath);
        if (is_dir($dir) || @mkdir($dir, 0770, true)) {
            ini_set('log_errors', '1');
            ini_set('error_log', $logPath);
        }
    }
}
