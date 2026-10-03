<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Catalog\CatalogSeed;
use Scutoons\Infrastructure\Config\Settings;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;

/**
 * The INSERTs of the catalog seed (server/config/*.json → catalog tables, agents/CATALOG.md) for the migration
 * 2026-10-04-catalog: printed, or – `--write` – put between the "BEGIN/END catalog seed" lines of both migration files.
 */
#[AsCommand(name: 'catalog:export-sql', description: 'Print the catalog seed (server/config/*.json) as INSERTs, or --write them into the 2026-10-04-catalog migrations')]
final class CatalogExportSqlCommand extends Command
{
    public const MIGRATION = '2026-10-04-catalog';
    private const BEGIN = '-- BEGIN catalog seed';
    private const END = '-- END catalog seed';

    public function __construct(private readonly CatalogSeed $seed, private readonly Settings $settings)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('driver', null, InputOption::VALUE_REQUIRED, 'mysql | sqlite', 'mysql')
            ->addOption('write', null, InputOption::VALUE_NONE, 'Replace the seed block of migrations/{mysql,sqlite}/' . self::MIGRATION . '.*.sql');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        if (!$input->getOption('write')) {
            $driver = (string) $input->getOption('driver');
            if (!in_array($driver, ['mysql', 'sqlite'], true)) {
                $output->writeln('<error>--driver is mysql or sqlite</error>');
                return self::FAILURE;
            }
            $output->writeln(implode("\n", $this->seed->inserts($driver)));
            return self::SUCCESS;
        }
        foreach (['mysql', 'sqlite'] as $driver) {
            $file = $this->settings->root() . "/migrations/$driver/" . self::MIGRATION . ".$driver.sql";
            $sql = (string) file_get_contents($file);
            $begin = strpos($sql, self::BEGIN);
            $end = strpos($sql, self::END);
            if ($begin === false || $end === false || $end < $begin) {
                $output->writeln("<error>$file has no seed block</error>");
                return self::FAILURE;
            }
            $head = substr($sql, 0, (int) strpos($sql, "\n", $begin) + 1);
            $inserts = $this->seed->inserts($driver);
            file_put_contents($file, $head . implode("\n", $inserts) . "\n" . substr($sql, $end));
            $output->writeln(sprintf('%s: %d INSERTs (one per table)', basename($file), count($inserts)));
        }
        return self::SUCCESS;
    }
}
