#!/usr/bin/env bash
# Smoke test of the accounts/items API (wrapper for tests/api-smoke.php).
# Needs the local server: composer serve (php -S localhost:9000 -t public public/router.php)
#   bash tests/api-smoke.sh [http://localhost:9000]
# Everything at once (unit tests + every contract suite against a started server on a DB copy): composer test
set -euo pipefail
cd "$(dirname "$0")/.."
BASE="${1:-http://localhost:9000}"
curl -fsS "$BASE/api/health/" > /dev/null || { echo "Server not reachable at $BASE (composer serve)"; exit 2; }
exec php tests/api-smoke.php "$BASE"
