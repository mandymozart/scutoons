<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\ApiError;
use Scutoons\Domain\Catalog\CatalogStore;
use Scutoons\Domain\Modules\ModuleCompiler;
use Scutoons\Domain\Modules\ModuleSource;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Input\InputOption;
use Symfony\Component\Console\Output\OutputInterface;

/**
 * The recompile list of the stored modules (#343, agents/research/ABI-V2.md §7: "a one-time command: rebuild --all over
 * the catalog's modules, the failures are the list"). A module compiled against another ABI (or not compiled since the
 * 2026-11-04-module-abi-v2 migration removed its v1 result) is compiled again from its own sources through the build
 * service – the same path as `POST /api/modules/compile { factory }` (ModuleCompiler), each result a new draft of
 * `module:<id>` to review and publish. A dry run by default; `--apply --as <creator>` compiles.
 */
#[AsCommand(name: 'modules:rebuild', description: 'List (and with --apply recompile) the catalog modules not built against the current module ABI')]
final class ModulesRebuildCommand extends Command
{
    public function __construct(private readonly CatalogStore $catalog, private readonly ModuleCompiler $compiler, private readonly Users $users)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addOption('all', null, InputOption::VALUE_NONE, 'every catalog module, not only those of another ABI')
            ->addOption('apply', null, InputOption::VALUE_NONE, 'compile (without it: the list only)')
            ->addOption('as', null, InputOption::VALUE_REQUIRED, 'the creator account the drafts are saved as (needed with --apply)')
            ->setHelp(<<<'HELP'
                  bin/console modules:rebuild                         the list: id, the ABI it was built against (or "not compiled")
                  bin/console modules:rebuild --all                   … every module
                  bin/console modules:rebuild --apply --as <email>    compile them; the failures are the list (exit 1)
                Production has no shell: there a creator compiles each listed module in the creator (or POST /api/modules/compile
                { "factory": "<id>" }); the list comes from the dry run against a copy of the data (npm run dev:db).
                HELP);
    }

    /**
     * The modules to rebuild: id → the ABI their stored descriptor names (null: not compiled)
     * @return array<string, ?int>
     */
    public function due(bool $all): array
    {
        $out = [];
        foreach ($this->catalog->map('modules') as $id => $module) {
            $abi = is_array($module) && is_array($module['descriptor'] ?? null) && is_int($module['descriptor']['abi'] ?? null) ? $module['descriptor']['abi'] : null;
            if ($all || $abi !== ModuleSource::ABI) $out[(string) $id] = $abi;
        }
        ksort($out);
        return $out;
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $due = $this->due((bool) $input->getOption('all'));
        $apply = (bool) $input->getOption('apply');
        if ($due === []) {
            $output->writeln('modules:rebuild: every catalog module is built against ABI ' . ModuleSource::ABI . '.');
            return self::SUCCESS;
        }
        $user = null;
        if ($apply) {
            $email = strtolower(trim((string) $input->getOption('as')));
            $user = $email !== '' ? $this->users->byEmail($email) : null;
            if (!$user) {
                $output->writeln('modules:rebuild --apply needs --as <creator email> (an account with the creator role).');
                return 2;
            }
        }
        $failures = [];
        foreach ($due as $id => $abi) {
            $was = $abi === null ? 'not compiled' : "ABI $abi";
            if (!$apply) {
                $output->writeln("$id\t$was\twould be recompiled");
                continue;
            }
            try {
                $r = $this->compiler->compile($user, null, $id);
                $output->writeln("$id\t$was\t→ ABI " . ($r['descriptor']['abi'] ?? '?') . ', ' . substr((string) $r['hash'], 0, 12) . ', draft ' . ($r['version'] ?? '?'));
            } catch (ApiError $e) {
                $failures[$id] = $e->getMessage();
                $output->writeln("$id\t$was\tFAILED: {$e->getMessage()}");
            }
        }
        $output->writeln(sprintf('modules:rebuild: %d module(s)%s%s', count($due), $apply ? ', ' . count($failures) . ' failed' : ' (dry run: --apply --as <email> compiles)', $failures ? ' – the failures use the removed ABI v1 API: deck/module/README.md' : ''));
        return $failures ? self::FAILURE : self::SUCCESS;
    }
}
