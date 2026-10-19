<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Infrastructure\Config\Settings;
use Scutoons\Infrastructure\Database\Database;
use Scutoons\Infrastructure\Database\MigrationRunner;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\ArrayInput;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\NullOutput;
use Symfony\Component\Console\Output\OutputInterface;

/**
 * One setup step for a fresh environment (BACKLOG 10.19, owner 2026-10-03: the cutover did these by hand and 6.9 found
 * the empty asset registry only through the creator's picker): the database schema, the content + catalog, the asset
 * registry, the content check and the settings – in this order, each one checked. Without --apply it only reports.
 */
#[AsCommand(name: 'setup', description: 'Check (or with --apply: do) what a fresh environment needs – migrations, content, asset registry, content check, settings')]
final class SetupCommand extends Command
{
    public function __construct(private readonly Database $db, private readonly Settings $settings)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('apply', null, InputOption::VALUE_NONE, 'Run the missing steps (migrate, assets:scan; with --seed also db:prod-seed)')
            ->addOption('seed', null, InputOption::VALUE_NONE, 'With --apply: load server/db/prod-seed.sql when there is no content (local development only)')
            ->setHelp(<<<'HELP'
                The steps, in order – each reports ok / missing / failed:
                  1. schema     the base schema and every dated migration (migrate)
                  2. content    stored chapters and the catalog (db:prod-seed locally; production: the export)
                  3. assets     the asset registry knows the media (assets:scan)
                  4. check      content:check – no stored document is broken or can get a reader stuck
                  5. settings   the settings a running API needs (CDN, mail sender, the log folder)
                Without --apply nothing changes. Production (no shell): run the pending migrations in phpMyAdmin, then this
                command's report is the checklist.
                HELP);
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $apply = (bool) $input->getOption('apply');
        /** every step's result (ok = true) */
        $ok = [];
        $output->writeln('Database: ' . (string) $this->db->value('SELECT DATABASE()') . ($apply ? '' : ' (report only – --apply to do the missing steps)'));

        // 1. schema
        $runner = new MigrationRunner($this->db, $this->settings->root() . '/migrations');
        $pending = ($runner->needsBaseSchema() ? 1 : 0) + count(array_filter($runner->status(), fn($at) => $at === null));
        if ($pending && $apply) {
            $this->runCommand('migrate', [], $output);
            $pending = ($runner->needsBaseSchema() ? 1 : 0) + count(array_filter($runner->status(), fn($at) => $at === null));
        }
        $ok[] = $this->report($output, 'schema', $pending === 0, $pending ? "$pending migration(s) pending" : 'every migration applied');
        if ($pending) return self::FAILURE; // the rest needs the tables

        // 2. content + catalog
        [$entities, $items] = [$this->count('entity_versions'), $this->count('catalog_items')];
        if (($entities === 0 || $items === 0) && $apply && $input->getOption('seed')) {
            $this->runCommand('db:prod-seed', [], $output);
            [$entities, $items] = [$this->count('entity_versions'), $this->count('catalog_items')];
        }
        $ok[] = $this->report($output, 'content', $entities > 0 && $items > 0, "$entities stored versions, $items catalog items" . ($entities && $items ? '' : ' – locally: --apply --seed (db:prod-seed); production: import the export'));

        // 3. asset registry
        $assets = $this->count('assets');
        if ($assets === 0 && $apply) {
            $this->runCommand('assets:scan', [], $output);
            $assets = $this->count('assets');
        }
        $ok[] = $this->report($output, 'assets', $assets > 0, $assets ? "$assets assets registered" : 'the registry is empty – assets:scan');

        // 4. content check (read-only)
        $check = $entities ? $this->runCommand('content:check', [], new NullOutput()) : 0;
        $ok[] = $this->report($output, 'check', $check === 0, $check === 0 ? 'every stored document is valid' : 'content:check found problems – run it for the list');

        // 5. settings
        $missing = array_values(array_filter(['CDN_URL', 'MAIL_FROM', 'LOG_PATH'], fn(string $key) => trim($this->settings->get($key)) === ''));
        $log = $this->settings->path('LOG_PATH');
        $logDir = $log !== '' ? dirname($log) : '';
        $writable = $logDir === '' || (is_dir($logDir) ? is_writable($logDir) : is_writable(dirname($logDir)));
        $ok[] = $this->report($output, 'settings', !$missing && $writable, $missing ? 'empty: ' . implode(', ', $missing) : ($writable ? 'set; the log folder is writable' : "the log folder $logDir is not writable"));

        $failed = in_array(false, $ok, true);
        $output->writeln($failed ? 'Not ready yet.' : 'Ready.');
        return $failed ? self::FAILURE : self::SUCCESS;
    }

    private function count(string $table): int
    {
        return (int) $this->db->value("SELECT COUNT(*) FROM `$table`");
    }

    /** @param array<string, mixed> $args */
    private function runCommand(string $command, array $args, OutputInterface $output): int
    {
        $application = $this->getApplication();
        if ($application === null) return 1;
        return $application->find($command)->run(new ArrayInput($args), $output);
    }

    private function report(OutputInterface $output, string $step, bool $ok, string $text): bool
    {
        $output->writeln(sprintf('  %-8s %-7s %s', $step, $ok ? 'ok' : 'missing', $text));
        return $ok;
    }
}
