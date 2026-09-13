#!/usr/bin/env bash
# Build a MariaDB (InnoDB) data directory loaded with the datacharmer/test_db "employees"
# sample database inside a digest-pinned official mariadb image (F08, kind build).
#
# ebrc script contract:
#   bash build_mariadb_test_db.sh --out <empty dir> --seed <int> --params <canonical JSON>
# Environment: EB_INPUT_GIT_ARCHIVE = git archive tar of github.com/datacharmer/test_db at
# the commit pinned in the source definition (acquired by provision.py).
# params (JSON object):
#   image     "docker.io/library/mariadb:<tag>@sha256:<digest>" (required)
#   platform  docker platform (default linux/amd64)
#   sql       top-level script to load (default "employees.sql")
#
# Procedure: docker run (entrypoint initializes the datadir) -> wait for TCP ping ->
# copy the test_db tree into the container -> mariadb < employees.sql -> SET GLOBAL
# innodb_fast_shutdown=0 (slow shutdown: purge + change-buffer merge) -> mariadb-admin
# shutdown -> docker cp /var/lib/mysql tar stream (uid/gid 999 kept) -> extract to
# <out>/mysql.  Not bit-reproducible (redo/undo LSNs, server UUID) -> output_pin null.
set -euo pipefail

OUT="" SEED=0 PARAMS="{}"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --out) OUT=$2; shift 2 ;;
    --seed) SEED=$2; shift 2 ;;
    --params) PARAMS=$2; shift 2 ;;
    *) echo "unknown arg $1" >&2; exit 2 ;;
  esac
done
[[ -n "$OUT" && -d "$OUT" ]] || { echo "--out must be an existing directory" >&2; exit 2; }
jqp() { python3 -c 'import json,sys; print(json.loads(sys.argv[1]).get(sys.argv[2], sys.argv[3]))' "$PARAMS" "$1" "$2"; }
IMAGE=$(jqp image "")
PLATFORM=$(jqp platform linux/amd64)
SQL=$(jqp sql employees.sql)
[[ "$IMAGE" =~ @sha256:[0-9a-f]{64}$ ]] || { echo "params.image must be pinned by digest" >&2; exit 2; }
[[ -f "${EB_INPUT_GIT_ARCHIVE:-}" ]] || { echo "EB_INPUT_GIT_ARCHIVE missing" >&2; exit 2; }
DOCKER=$(command -v docker) || { echo "docker CLI not found" >&2; exit 2; }
PW=ebrc-corpus-not-secret

SRC="${EB_SCRATCH:-$(mktemp -d)}/test_db"
rm -rf "$SRC"; mkdir -p "$SRC"
tar -x -f "$EB_INPUT_GIT_ARCHIVE" -C "$SRC"
[[ -f "$SRC/$SQL" ]] || { echo "$SQL not found in test_db archive" >&2; exit 1; }

NAME="ebrc-mariadb-${EB_ITEM_ID:-item}-$$-$RANDOM"
cleanup() { "$DOCKER" rm -f -v "$NAME" >/dev/null 2>&1 || true; }
trap cleanup EXIT

"$DOCKER" pull --platform "$PLATFORM" "$IMAGE" >/dev/null
"$DOCKER" run -d --name "$NAME" --platform "$PLATFORM" -e MARIADB_ROOT_PASSWORD="$PW" \
  "$IMAGE" --character-set-server=utf8mb4 --collation-server=utf8mb4_general_ci >/dev/null
for i in $(seq 1 300); do
  if "$DOCKER" exec "$NAME" mariadb-admin --protocol=tcp -h 127.0.0.1 -uroot -p"$PW" ping >/dev/null 2>&1; then break; fi
  if [[ "$("$DOCKER" inspect -f '{{.State.Running}}' "$NAME")" != "true" ]]; then
    "$DOCKER" logs "$NAME" >&2 || true; echo "mariadb container exited" >&2; exit 1
  fi
  sleep 1
done
"$DOCKER" exec "$NAME" mariadb-admin --protocol=tcp -h 127.0.0.1 -uroot -p"$PW" ping >/dev/null
SERVER_VERSION=$("$DOCKER" exec "$NAME" mariadbd --version)
echo "server: $SERVER_VERSION"

"$DOCKER" exec "$NAME" mkdir -p /tmp/ebrc
"$DOCKER" cp "$SRC" "$NAME:/tmp/ebrc/"
"$DOCKER" exec -w /tmp/ebrc/test_db "$NAME" sh -c "mariadb -uroot -p'$PW' < '$SQL'"
"$DOCKER" exec "$NAME" mariadb -uroot -p"$PW" -N -e \
  "SELECT 'employees', COUNT(*) FROM employees.employees UNION ALL SELECT 'salaries', COUNT(*) FROM employees.salaries"
"$DOCKER" exec "$NAME" rm -rf /tmp/ebrc
"$DOCKER" exec "$NAME" mariadb -uroot -p"$PW" -e "SET GLOBAL innodb_fast_shutdown=0"
# mariadbd is PID 1, so the exec session is killed (exit 137) when the server exits;
# success is judged by the container stopping with a "Shutdown complete" log line.
"$DOCKER" exec "$NAME" mariadb-admin -uroot -p"$PW" shutdown || true
for i in $(seq 1 600); do
  [[ "$("$DOCKER" inspect -f '{{.State.Running}}' "$NAME")" == "true" ]] || break
  sleep 1
done
"$DOCKER" logs "$NAME" 2>&1 | tail -4
"$DOCKER" logs "$NAME" 2>&1 | grep -q "Shutdown complete" || { echo "mariadb did not report a clean shutdown" >&2; exit 1; }

mkdir -p "$OUT/mysql"
"$DOCKER" cp "$NAME:/var/lib/mysql" - | tar -x -f - -C "$OUT/mysql" --strip-components=1 --numeric-owner --same-owner --same-permissions
python3 - "$OUT/EBRC-BUILD-INFO.json" "$IMAGE" "$PLATFORM" "$SERVER_VERSION" "$SQL" <<'PY'
import json, sys
path, image, platform, server, sql = sys.argv[1:]
with open(path, "w") as f:
    json.dump({"builder": "research/corpus/generators/g3-data/build_mariadb_test_db.sh", "image": image, "platform": platform,
               "server_version": server, "loaded_script": sql, "shutdown": "innodb_fast_shutdown=0 then mariadb-admin shutdown",
               "layout": "mysql/ is the MariaDB datadir (owner uid/gid 999 as in the image)"}, f, indent=1, sort_keys=True)
    f.write("\n")
PY
chmod 0644 "$OUT/EBRC-BUILD-INFO.json"
