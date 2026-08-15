#!/usr/bin/env sh
# Issue a local CA and the server/client certificates the secure sample
# containers need.
#
# Nothing here is a secret worth protecting — every key is generated on demand,
# lives under tls/certs/ (gitignored), and is only ever trusted by the sample
# containers on this machine. That is the point: TLS paths cannot be tested
# without certificates, and certificates that ship in a repository teach the
# wrong habit.
#
# Usage: sh tls/generate-certs.sh [output-dir]
set -eu

OUT="${1:-$(dirname "$0")/certs}"
DAYS=825            # the maximum a modern client will accept for a leaf
CLIENT_CN=irodori_cert  # must equal the database role that authenticates by cert

if [ -f "$OUT/ca.crt" ] && [ -f "$OUT/server.crt" ] && [ -f "$OUT/client.crt" ]; then
  echo "certificates already present in $OUT (delete the directory to reissue)"
  exit 0
fi

mkdir -p "$OUT"
cd "$OUT"

echo "issuing a local CA"
openssl req -x509 -newkey rsa:2048 -nodes -keyout ca.key -out ca.crt \
  -days "$DAYS" -subj "/CN=Irodori Samples Local CA" 2>/dev/null

# The server certificate has to carry both names: `verify-full` checks the
# hostname, and the sample URLs use localhost while some drivers resolve it to
# 127.0.0.1 before the check.
echo "issuing a server certificate for localhost/127.0.0.1"
openssl req -newkey rsa:2048 -nodes -keyout server.key -out server.csr \
  -subj "/CN=localhost" 2>/dev/null
printf 'subjectAltName=DNS:localhost,IP:127.0.0.1\nextendedKeyUsage=serverAuth\n' > server.ext
openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial \
  -out server.crt -days "$DAYS" -extfile server.ext 2>/dev/null

# The client certificate's CN is the database username. Postgres `cert` auth and
# MongoDB's MONGODB-X509 both derive the identity from the subject, so a
# mismatch here fails with an error about the *user*, not the certificate.
echo "issuing a client certificate for CN=$CLIENT_CN"
openssl req -newkey rsa:2048 -nodes -keyout client.key -out client.csr \
  -subj "/CN=$CLIENT_CN" 2>/dev/null
printf 'extendedKeyUsage=clientAuth\n' > client.ext
openssl x509 -req -in client.csr -CA ca.crt -CAkey ca.key -CAcreateserial \
  -out client.crt -days "$DAYS" -extfile client.ext 2>/dev/null

# native-tls (QuestDB's stack, and anything using the Windows or macOS backend)
# accepts only PKCS#8 client keys and rejects the PKCS#1 form openssl writes by
# default on some versions. Provide both so a connector can use either.
openssl pkcs8 -topk8 -nocrypt -in client.key -out client.pk8.key 2>/dev/null

# A key readable by anyone is refused outright by several servers.
chmod 600 ca.key server.key client.key client.pk8.key
rm -f server.csr client.csr server.ext client.ext

echo "done: $(pwd)"
ls -1
