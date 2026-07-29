#!/bin/bash
# Creates a stable self-signed code-signing identity for TriOS development.
#
# WHY THIS EXISTS
# ---------------
# build.sh signs the app ad-hoc (`codesign --sign -`). An ad-hoc signature has no
# stable identity: every rebuild produces a new code directory hash, so macOS
# treats each build as a *different application*. Keychain items are bound to the
# application that created them, so after every rebuild the login keychain asks
# for your password again - once per service (encryption-key, local-auth,
# model-keys), which is why several dialogs appear in a row.
#
# Signing with a stable certificate fixes that at the root: the identity stops
# changing, so clicking "Always Allow" once actually sticks. It does NOT weaken
# any keychain protection - the secrets stay exactly as protected as before.
#
# This script needs your login keychain password, so run it yourself:
#   bash scripts/create_dev_signing_identity.sh
#
# Afterwards, build with:
#   TRIOS_SIGN_IDENTITY="TriOS Development" ./build.sh
#
# To undo: delete the "TriOS Development" certificate in Keychain Access.

set -e

IDENTITY_NAME="${1:-TriOS Development}"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

if security find-identity -v -p codesigning | grep -q "$IDENTITY_NAME"; then
    echo "[OK] Signing identity already exists: $IDENTITY_NAME"
    exit 0
fi

echo "Creating self-signed code-signing certificate: $IDENTITY_NAME"

cat > "$WORK_DIR/openssl.cnf" <<'CONF'
[ req ]
distinguished_name = dn
x509_extensions    = ext
prompt             = no

[ dn ]
CN = PLACEHOLDER_CN

[ ext ]
basicConstraints     = critical,CA:false
keyUsage             = critical,digitalSignature
extendedKeyUsage     = critical,codeSigning
CONF
sed -i '' "s/PLACEHOLDER_CN/$IDENTITY_NAME/" "$WORK_DIR/openssl.cnf"

openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
    -keyout "$WORK_DIR/key.pem" -out "$WORK_DIR/cert.pem" \
    -config "$WORK_DIR/openssl.cnf" 2>/dev/null

openssl pkcs12 -export -inkey "$WORK_DIR/key.pem" -in "$WORK_DIR/cert.pem" \
    -out "$WORK_DIR/identity.p12" -passout pass: 2>/dev/null

KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"

# -T grants codesign access to the private key without a prompt per signature.
security import "$WORK_DIR/identity.p12" -k "$KEYCHAIN" -P "" \
    -T /usr/bin/codesign -T /usr/bin/security

# Mark the certificate as trusted for code signing.
security add-trusted-cert -d -r trustAsRoot -p codeSign -k "$KEYCHAIN" "$WORK_DIR/cert.pem"

# Allow codesign to use the key non-interactively from now on.
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "" "$KEYCHAIN" >/dev/null 2>&1 || \
    echo "[WARN] set-key-partition-list needs your login password; codesign may prompt once."

echo "[OK] Created signing identity: $IDENTITY_NAME"
security find-identity -v -p codesigning | grep "$IDENTITY_NAME" || true
echo
echo "Now rebuild with:"
echo "  TRIOS_SIGN_IDENTITY=\"$IDENTITY_NAME\" ./build.sh"
echo
echo "On the next launch, click \"Always Allow\" once per keychain dialog."
echo "Because the identity is now stable, those dialogs will not come back."
