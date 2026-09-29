<?php
/**
 * Account roles from the command line (ITEMS.md "Roles, signup rule, promo codes").
 *
 *   php scripts/roles.php grant  <email> <role>     e.g. grant someone@example.com creator
 *   php scripts/roles.php revoke <email> <role>
 *   php scripts/roles.php list   <email>            the roles of one account
 *   php scripts/roles.php list                      every account with a role other than member
 *
 * Roles: member (every account), admin (reserved), creator (scutoons.com/creator + content API).
 * Uses the configured database (config.local.php locally). Production (no shell): the admin SPA "Users" view.
 */
if (PHP_SAPI !== 'cli') {
    http_response_code(404);
    exit;
}

require_once __DIR__ . '/../api/lib/bootstrap.php';

function usage(): never
{
    fwrite(STDERR, "Usage: php scripts/roles.php grant|revoke <email> <role>\n       php scripts/roles.php list [email]\n");
    fwrite(STDERR, 'Roles: ' . implode(', ', array_column(Roles::all(), 'name')) . "\n");
    exit(2);
}

$command = $argv[1] ?? '';
$email = isset($argv[2]) ? strtolower(trim($argv[2])) : null;
$role = $argv[3] ?? null;

if ($command === 'list' && $email === null) {
    $rows = Db::all("SELECT u.email, r.name, ur.granted_at, ur.granted_by FROM user_roles ur JOIN users u ON u.id = ur.user_id
                     JOIN roles r ON r.id = ur.role_id WHERE r.name <> 'member' ORDER BY r.id, u.email");
    if (!$rows) echo "No account has a role other than member.\n";
    foreach ($rows as $row) echo str_pad($row['name'], 9) . $row['email'] . "  (since {$row['granted_at']} UTC" . ($row['granted_by'] ? ", by {$row['granted_by']}" : '') . ")\n";
    exit(0);
}
if (!in_array($command, ['grant', 'revoke', 'list'], true) || $email === null) usage();

$user = Users::byEmail($email);
if (!$user) {
    fwrite(STDERR, "No account with the email $email.\n");
    exit(1);
}
if ($command === 'list') {
    echo $email . ': ' . (implode(', ', Roles::of($user['id'])) ?: '(no roles)') . "\n";
    exit(0);
}
if ($role === null) usage();
if (!Roles::exists($role)) {
    fwrite(STDERR, "Unknown role $role. Roles: " . implode(', ', array_column(Roles::all(), 'name')) . "\n");
    exit(1);
}
$changed = $command === 'grant' ? Roles::grant($user['id'], $role, 'cli') : Roles::revoke($user['id'], $role);
$message = match (true) {
    $command === 'grant' && $changed => "Granted $role to $email.",
    $command === 'grant' => "$email already has $role.",
    $changed => "Revoked $role from $email.",
    default => "$email did not have $role.",
};
echo $message . ' Roles now: ' . (implode(', ', Roles::of($user['id'])) ?: '(none)') . "
";
