<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Content\ContentSchemas;
use Scutoons\Infrastructure\Database\Database;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;

#[AsCommand(name: 'content:check', description: 'READ-ONLY: checks the newest and the published version of every stored entity against its type (ContentSchemas)')]
final class ContentCheckCommand extends Command
{
    public function __construct(private readonly Database $db, private readonly ContentSchemas $schemas)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('db', null, InputOption::VALUE_REQUIRED, 'Another SQLite database file')
            ->addOption('kind', null, InputOption::VALUE_REQUIRED, 'Only this entity kind')
            ->addOption('limit', null, InputOption::VALUE_REQUIRED, 'Problems shown per document', '5')
            ->setHelp('Exit code 1 when a document does not fit. Saves are refused with 422 invalid_content for the same problems.');
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $dbFile = $input->getOption('db');
        $db = is_string($dbFile) ? Database::sqlite($dbFile) : $this->db;
        $kind = $input->getOption('kind');
        $limit = max(1, (int) $input->getOption('limit'));

        $where = is_string($kind) ? 'WHERE kind = ?' : '';
        $rows = $db->all("SELECT v.id, v.entity_key, v.kind, v.status, v.json FROM entity_versions v
            JOIN (SELECT entity_key, MAX(id) AS latest, MAX(CASE WHEN status = 'published' THEN id END) AS published
                  FROM entity_versions $where GROUP BY entity_key) g ON v.id = g.latest OR v.id = g.published
            ORDER BY v.kind, v.entity_key, v.id", is_string($kind) ? [$kind] : []);

        $bad = 0;
        $perKind = [];
        foreach ($rows as $row) {
            $k = (string) $row['kind'];
            $perKind[$k] ??= ['checked' => 0, 'bad' => 0, 'unchecked' => !(ContentSchemas::BY_KIND[$k] ?? null)];
            $perKind[$k]['checked']++;
            $problems = $this->schemas->problems($k, json_decode((string) $row['json'], false), $limit);
            if (!$problems) continue;
            $bad++;
            $perKind[$k]['bad']++;
            $output->writeln("{$row['entity_key']} (version {$row['id']}, {$row['status']}):");
            foreach ($problems as $problem) $output->writeln("    $problem");
        }

        $output->writeln('');
        foreach ($perKind as $k => $s) {
            $output->writeln(sprintf('  %-15s %4d checked  %s', $k, $s['checked'], $s['unchecked'] ? 'no schema (not checked)' : ($s['bad'] ? "{$s['bad']} do not fit" : 'ok')));
        }
        $output->writeln($bad ? "$bad version(s) do not fit their type." : 'Every checked version fits its type.');
        return $bad ? 1 : 0;
    }
}
