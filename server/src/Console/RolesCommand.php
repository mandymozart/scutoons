<?php
declare(strict_types=1);

namespace Scutoons\Console;

use Scutoons\Domain\Accounts\Users;
use Scutoons\Domain\Roles\Roles;
use Scutoons\Infrastructure\Database\Database;
use Symfony\Component\Console\Attribute\AsCommand;
use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputArgument;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\ConsoleOutputInterface;
use Symfony\Component\Console\Output\OutputInterface;

#[AsCommand(name: 'roles', description: 'Grant, revoke or list account roles (member, admin, creator)')]
final class RolesCommand extends Command
{
    public function __construct(private readonly Roles $roles, private readonly Users $users, private readonly Database $db)
    {
        parent::__construct();
    }

    protected function configure(): void
    {
        $this->addArgument('action', InputArgument::REQUIRED, 'grant | revoke | list')
            ->addArgument('email', InputArgument::OPTIONAL, 'The account')
            ->addArgument('role', InputArgument::OPTIONAL, 'The role (grant/revoke)')
            ->setHelp(<<<'HELP'
                  bin/console roles grant  <email> <role>     e.g. roles grant someone@example.com admin
                  bin/console roles revoke <email> <role>
                  bin/console roles list   <email>            the roles of one account
                  bin/console roles list                      every account with a role other than member
                Production (no shell): the admin app (Users) or the legacy generate/ "Users" view.
                HELP);
    }

    protected function execute(InputInterface $input, OutputInterface $output): int
    {
        $errors = $output instanceof ConsoleOutputInterface ? $output->getErrorOutput() : $output;
        $action = (string) $input->getArgument('action');
        $email = $input->getArgument('email') !== null ? strtolower(trim((string) $input->getArgument('email'))) : null;
        $role = $input->getArgument('role');
        $names = implode(', ', array_column($this->roles->all(), 'name'));

        if ($action === 'list' && $email === null) {
            $rows = $this->db->all("SELECT u.email, r.name, ur.granted_at, ur.granted_by FROM user_roles ur JOIN users u ON u.id = ur.user_id
                                   JOIN roles r ON r.id = ur.role_id WHERE r.name <> 'member' ORDER BY r.id, u.email");
            if (!$rows) $output->writeln('No account has a role other than member.');
            foreach ($rows as $row) {
                $output->writeln(str_pad((string) $row['name'], 9) . $row['email'] . "  (since {$row['granted_at']} UTC" . ($row['granted_by'] ? ", by {$row['granted_by']}" : '') . ')');
            }
            return self::SUCCESS;
        }
        if (!in_array($action, ['grant', 'revoke', 'list'], true) || $email === null) {
            $errors->writeln("Usage: bin/console roles grant|revoke <email> <role>\n       bin/console roles list [email]\nRoles: $names");
            return 2;
        }
        $user = $this->users->byEmail($email);
        if (!$user) {
            $errors->writeln("No account with the email $email.");
            return self::FAILURE;
        }
        $userId = (string) $user['id'];
        if ($action === 'list') {
            $output->writeln($email . ': ' . (implode(', ', $this->roles->of($userId)) ?: '(no roles)'));
            return self::SUCCESS;
        }
        if (!is_string($role)) {
            $errors->writeln("Usage: bin/console roles $action <email> <role>\nRoles: $names");
            return 2;
        }
        if (!$this->roles->exists($role)) {
            $errors->writeln("Unknown role $role. Roles: $names");
            return self::FAILURE;
        }
        $changed = $action === 'grant' ? $this->roles->grant($userId, $role, 'cli') : $this->roles->revoke($userId, $role);
        $message = match (true) {
            $action === 'grant' && $changed => "Granted $role to $email.",
            $action === 'grant' => "$email already has $role.",
            $changed => "Revoked $role from $email.",
            default => "$email did not have $role.",
        };
        $output->writeln($message . ' Roles now: ' . (implode(', ', $this->roles->of($userId)) ?: '(none)'));
        return self::SUCCESS;
    }
}
