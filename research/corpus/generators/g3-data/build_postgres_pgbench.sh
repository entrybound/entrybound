#!/usr/bin/env bash
# Build a PostgreSQL data directory initialized by `pgbench -i` inside a digest-pinned
# official postgres image (F08, kind build).  g3-data build script.
#
# ebrc script contract:
#   bash build_postgres_pgbench.sh --out <empty dir> --seed <int> --params <canonical JSON>
# params (JSON object):
#   image     "docker.io/library/postgres:<tag>@sha256:<digest>" (required; digest pinned)
#   platform  docker platform (default linux/amd64)
#   scale     pgbench scale factor (default 10; ~15 MiB of table data per unit)
#   initdb_args  extra POSTGRES_INITDB_ARGS (default "--locale=C --encoding=UTF8")
#
# Procedure: docker run (entrypoint runs initdb and starts the server) -> wait until the
# final server accepts TCP connections -> pgbench -i -s <scale> -> CHECKPOINT -> clean
# shutdown via `docker stop` (postgres image STOPSIGNAL SIGINT = fast shutdown with a
# shutdown checkpoint) -> `docker cp` of PGDATA as a tar stream (keeps uid/gid 999 and
# modes) -> extract into --out.  The container and its anonymous volume are always removed.
#
# Not bit-reproducible (system identifier, xids/LSN timing, pg_control timestamps), so the
# item uses output_pin null.  Layout: <out>/pgdata/ is PGDATA; <out>/EBRC-BUILD-INFO.json
# (next to, not inside, PGDATA) records the image digest, scale, server/pgbench versions and
# the pg_controldata cluster state.
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
jqp() { python3 -c 'import json,sys; v=json.loads(sys.argv[1]).get(sys.argv[2], sys.argv[3]); print(v)' "$PARAMS" "$1" "$2"; }
IMAGE=$(jqp image "")
PLATFORM=$(jqp platform linux/amd64)
SCALE=$(jqp scale 10)
INITDB_ARGS=$(jqp initdb_args "--locale=C --encoding=UTF8")
[[ "$IMAGE" =~ @sha256:[0-9a-f]{64}$ ]] || { echo "params.image must be pinned by digest" >&2; exit 2; }
[[ "$SCALE" =~ ^[0-9]+$ ]] || { echo "params.scale must be an integer" >&2; exit 2; }
DOCKER=$(command -v docker) || { echo "docker CLI not found" >&2; exit 2; }

NAME="ebrc-pgbench-${EB_ITEM_ID:-item}-$$-$RANDOM"
cleanup() { "$DOCKER" rm -f -v "$NAME" >/dev/null 2>&1 || true; }
trap cleanup EXIT

"$DOCKER" pull --platform "$PLATFORM" "$IMAGE" >/dev/null
"$DOCKER" run -d --name "$NAME" --platform "$PLATFORM" \
  -e POSTGRES_PASSWORD=ebrc-corpus-not-secret -e POSTGRES_INITDB_ARGS="$INITDB_ARGS" \
  -e PGDATA=/var/lib/postgresql/data "$IMAGE" >/dev/null

# The entrypoint's temporary init server listens on the unix socket only; a TCP
# connection therefore proves the final server is up.
for i in $(seq 1 300); do
  if "$DOCKER" exec "$NAME" pg_isready -q -h 127.0.0.1 -U postgres 2>/dev/null; then break; fi
  if [[ "$("$DOCKER" inspect -f '{{.State.Running}}' "$NAME")" != "true" ]]; then
    "$DOCKER" logs "$NAME" >&2 || true; echo "postgres container exited" >&2; exit 1
  fi
  sleep 1
done
"$DOCKER" exec "$NAME" pg_isready -q -h 127.0.0.1 -U postgres

PG_VERSION=$("$DOCKER" exec "$NAME" postgres --version)
PGBENCH_VERSION=$("$DOCKER" exec "$NAME" pgbench --version)
echo "server: $PG_VERSION; $PGBENCH_VERSION; scale=$SCALE"
"$DOCKER" exec -e PGPASSWORD=ebrc-corpus-not-secret "$NAME" pgbench -h 127.0.0.1 -U postgres -i -s "$SCALE" -q postgres
"$DOCKER" exec "$NAME" psql -h 127.0.0.1 -U postgres -d postgres -v ON_ERROR_STOP=1 -Atc \
  "SELECT 'accounts='||count(*) FROM pgbench_accounts"
"$DOCKER" exec "$NAME" psql -h 127.0.0.1 -U postgres -d postgres -v ON_ERROR_STOP=1 -c "CHECKPOINT"
"$DOCKER" stop -t 600 "$NAME" >/dev/null
EXIT_CODE=$("$DOCKER" inspect -f '{{.State.ExitCode}}' "$NAME")
[[ "$EXIT_CODE" == "0" ]] || { "$DOCKER" logs "$NAME" >&2; echo "postgres did not shut down cleanly (exit $EXIT_CODE)" >&2; exit 1; }
"$DOCKER" logs "$NAME" 2>&1 | tail -5

# pg_controldata (run against the stopped container's volume) must report a clean shutdown
STATE=$("$DOCKER" run --rm --platform "$PLATFORM" --volumes-from "$NAME" --user postgres --entrypoint pg_controldata \
        "$IMAGE" /var/lib/postgresql/data | awk -F': *' '/Database cluster state/{print $2}')
echo "pg_controldata cluster state: $STATE"
[[ "$STATE" == "shut down" ]] || { echo "unexpected cluster state '$STATE'" >&2; exit 1; }

mkdir -p "$OUT/pgdata"
"$DOCKER" cp "$NAME:/var/lib/postgresql/data" - | tar -x -f - -C "$OUT/pgdata" --strip-components=1 --numeric-owner --same-owner --same-permissions
chmod 0700 "$OUT/pgdata"
python3 - "$OUT/EBRC-BUILD-INFO.json" "$IMAGE" "$PLATFORM" "$SCALE" "$PG_VERSION" "$PGBENCH_VERSION" "$INITDB_ARGS" "$STATE" <<'PY'
import json, sys
path, image, platform, scale, pgv, pgbv, initdb, state = sys.argv[1:]
json.dump({"builder": "research/corpus/generators/g3-data/build_postgres_pgbench.sh", "image": image, "platform": platform,
           "pgbench_scale": int(scale), "server_version": pgv, "pgbench_version": pgbv, "initdb_args": initdb,
           "cluster_state_after_build": state, "layout": "pgdata/ is the PGDATA directory (owner uid/gid 999 as in the image)"},
          open(path, "w"), indent=1, sort_keys=True)
open(path, "a").write("\n")
PY
chmod 0644 "$OUT/EBRC-BUILD-INFO.json"
