<?php
declare(strict_types=1);

/**
 * PHP-DI definitions. Everything not listed here is autowired (constructor type hints).
 * Settings is added by Scutoons\Kernel (loaded once per request / command).
 */

use Psr\Container\ContainerInterface;
use Psr\Http\Message\ResponseFactoryInterface;
use Scutoons\Domain\RequestContext;
use Scutoons\Infrastructure\AssetStorage\AssetStorages;
use Scutoons\Infrastructure\Cache\ApcuCache;
use Scutoons\Infrastructure\Cache\Cache;
use Scutoons\Infrastructure\Cache\FileCache;
use Scutoons\Infrastructure\Cache\NullCache;
use Scutoons\Infrastructure\Clock\Clock;
use Scutoons\Infrastructure\Clock\SystemClock;
use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Signing\KeyStore;
use Scutoons\Infrastructure\Storage\FileStore;
use Scutoons\Infrastructure\Templates\Templates;
use Slim\Psr7\Factory\ResponseFactory;

return [
    Clock::class => DI\get(SystemClock::class),
    RequestContext::class => DI\create(RequestContext::class),
    ResponseFactoryInterface::class => DI\get(ResponseFactory::class),

    // SQLite when DB_SQLITE_PATH is set (local development, tests), else MySQL/MariaDB (production)
    Database::class => static function (ContainerInterface $c): Database {
        $settings = $c->get(Settings::class);
        $sqlite = $settings->path('DB_SQLITE_PATH');
        if ($sqlite !== '') return Database::sqlite($sqlite);
        return Database::mysql($settings->get('DB_HOST'), $settings->get('DB_NAME'), $settings->get('DB_USER'), $settings->get('DB_PASS'), $settings->get('DB_PORT'));
    },

    Templates::class => static fn(ContainerInterface $c): Templates => new Templates($c->get(Settings::class)->root() . '/templates'),
    KeyStore::class => static fn(ContainerInterface $c): KeyStore => new KeyStore($c->get(Settings::class)->path('KEYPAIR_PATH')),
    // media storages of the asset layer (agents/ASSETS.md): local CDN folder, bunny.net
    AssetStorages::class => static fn(ContainerInterface $c): AssetStorages => new AssetStorages($c->get(Settings::class)),
    FileStore::class => static fn(ContainerInterface $c): FileStore => new FileStore($c->get(Settings::class)->path('FILES_PATH')),

    // Composed library answers (agents/DATA.md §3): APCu when the host has it, else files in CACHE_PATH
    Cache::class => static function (ContainerInterface $c): Cache {
        $settings = $c->get(Settings::class);
        $driver = strtolower($settings->get('CACHE_DRIVER')) ?: 'auto';
        $path = $settings->path('CACHE_PATH');
        return match (true) {
            $driver === 'none' => new NullCache(),
            $driver === 'apcu' || ($driver === 'auto' && ApcuCache::available()) => ApcuCache::available() ? new ApcuCache() : new NullCache(),
            $path === '' => new NullCache(),
            default => new FileCache($path . '/library'),
        };
    },
];
