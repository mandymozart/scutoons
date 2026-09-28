#!/usr/bin/env bash
# Smoke test of the accounts/items API (wrapper for tests/api-smoke.php).
# Needs the local server: php -S localhost:9000 router.php
#   bash tests/api-smoke.sh [http://localhost:9000]
set -euo pipefail
cd "$(dirname "$0")/.."
BASE="${1:-http://localhost:9000}"
curl -fsS "$BASE/api/health/" > /dev/null || { echo "Server not reachable at $BASE (php -S localhost:9000 router.php)"; exit 2; }
exec php tests/api-smoke.php "$BASE"
