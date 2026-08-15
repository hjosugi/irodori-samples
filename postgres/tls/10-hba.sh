#!/bin/sh
# Replace pg_hba so the TLS sample actually requires TLS.
#
# `hostssl` refuses a plaintext connection outright, which is the behaviour
# worth testing: with the default `host` rules a client asking for sslmode=require
# and a client asking for nothing both succeed, and the sample proves nothing.
set -eu

cat > "$PGDATA/pg_hba.conf" <<'HBA'
# TYPE   DATABASE  USER          ADDRESS       METHOD
local    all       all                         trust

# Certificate authentication first: a client presenting one authenticates as the
# CN in its subject, with no password involved.
hostssl  all       irodori_cert  all       cert clientcert=verify-full

# Everyone else over TLS, with a password, using scram.
hostssl  all       all           all           scram-sha-256

# No `host` line at all: a plaintext connection is rejected rather than quietly
# accepted, which is the whole point of this container.
HBA
