#!/usr/bin/env bash
#
# Apply a committed seed to an engine whose image has no init hook.
#
#   scripts/seed.sh redis
#   scripts/seed.sh all
#
# Most engines here need nothing: PostgreSQL, MySQL, MariaDB, MongoDB, Oracle
# and ClickHouse all read `01_samples.*` from their image's init directory, so
# `compose up` alone leaves them populated. The ones below have no such hook, so
# the seed is pushed in after the container answers.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENGINE_BIN="${ENGINE_BIN:-$(command -v podman >/dev/null 2>&1 && echo podman || echo docker)}"

# Engines that seed themselves on first boot.
SELF_SEEDING=(postgres mysql mariadb mongodb oracle clickhouse)
# Engines this script handles.
MANUAL=(redis neo4j memgraph cassandra scylladb sqlite duckdb)

GREEN=$'\033[32m'; RED=$'\033[31m'; DIM=$'\033[2m'; BOLD=$'\033[1m'; OFF=$'\033[0m'
info() { printf '%s==>%s %s\n' "$BOLD" "$OFF" "$*"; }
ok()   { printf '%s  ok%s %s\n' "$GREEN" "$OFF" "$*"; }
fail() { printf '%s FAIL%s %s\n' "$RED" "$OFF" "$*"; }

# `podman compose` names containers <project>_<service>_1; resolve rather than
# guess, so a rename upstream does not silently break this.
container() {
  "$ENGINE_BIN" ps --format '{{.Names}}' 2>/dev/null | grep -m1 -E "^irodori-$1[_-]" || true
}

seed_one() {
  local engine="$1" c
  case " ${SELF_SEEDING[*]} " in
    *" $engine "*)
      ok "$engine seeds itself from its init hook — nothing to do"
      return 0 ;;
  esac

  case "$engine" in
    sqlite)
      command -v sqlite3 >/dev/null || { fail "sqlite3 not on PATH"; return 1; }
      rm -f "$ROOT/sqlite/samples.db"
      sqlite3 "$ROOT/sqlite/samples.db" < "$ROOT/sqlite/01_samples.sql" || return 1
      ok "sqlite -> sqlite/samples.db"
      return 0 ;;
    duckdb)
      command -v duckdb >/dev/null || { fail "duckdb not on PATH"; return 1; }
      rm -f "$ROOT/duckdb/samples.duckdb"
      duckdb "$ROOT/duckdb/samples.duckdb" < "$ROOT/duckdb/01_samples.sql" || return 1
      ok "duckdb -> duckdb/samples.duckdb"
      return 0 ;;
  esac

  c="$(container "$engine")"
  if [ -z "$c" ]; then
    fail "$engine is not running — 'podman compose -f $engine/compose.yaml up -d' first"
    return 1
  fi

  case "$engine" in
    redis)
      # redis-cli reading stdin runs one command per line, which is what the
      # seed file is. The `#` header lines are dropped first: redis-cli would
      # answer each one with an unknown-command error.
      grep -v '^#' "$ROOT/redis/01_samples.redis" \
        | "$ENGINE_BIN" exec -i "$c" sh -c 'redis-cli -a irodori --no-auth-warning 2>/dev/null' >/dev/null || return 1
      ok "redis: $("$ENGINE_BIN" exec "$c" sh -c 'redis-cli -a irodori --no-auth-warning DBSIZE 2>/dev/null') keys" ;;
    neo4j)
      "$ENGINE_BIN" exec -i "$c" cypher-shell -u neo4j -p irodoripass --format plain \
        < "$ROOT/neo4j/01_samples.cypher" >/dev/null || return 1
      ok "neo4j: $("$ENGINE_BIN" exec "$c" cypher-shell -u neo4j -p irodoripass --format plain \
        'match (n) return count(n)' 2>/dev/null | tail -1) nodes" ;;
    memgraph)
      "$ENGINE_BIN" exec -i "$c" mgconsole < "$ROOT/memgraph/01_samples.cypher" >/dev/null || return 1
      ok "memgraph: $("$ENGINE_BIN" exec "$c" sh -c \
        'echo "MATCH (n) RETURN Count(n);" | mgconsole --output-format=csv' 2>/dev/null | tail -1) nodes" ;;
    cassandra|scylladb)
      "$ENGINE_BIN" exec -i "$c" cqlsh --request-timeout=120 \
        < "$ROOT/cassandra/01_samples.cql" >/dev/null || return 1
      ok "$engine: keyspace samples loaded" ;;
    *)
      fail "no seed step for '$engine'"
      return 2 ;;
  esac
}

case "${1:-help}" in
  all)
    rc=0
    for e in "${MANUAL[@]}"; do info "seed $e"; seed_one "$e" || rc=1; done
    exit $rc ;;
  help|-h|--help)
    sed -n '3,13p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    echo
    echo "self-seeding: ${SELF_SEEDING[*]}"
    echo "manual:       ${MANUAL[*]}" ;;
  *)
    info "seed $1"; seed_one "$1" ;;
esac
