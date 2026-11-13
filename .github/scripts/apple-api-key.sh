#!/usr/bin/env bash
# The App Store Connect API key for notarising the Mac app (docs/UPDATES.md "Signing the Mac app"),
# written to the file named in $1 as the PEM Tauri reads (APPLE_API_KEY_PATH), however the secret
# APPLE_API_KEY_P8 was pasted: the .p8 file as it is, the same on one line (GitHub's secret box or a
# copy can lose its line breaks), the key's base64 without its BEGIN/END lines, or the whole file in
# base64. Then checked, so a key that is no key stops here with what is wrong, not in Apple's
# notarisation as `invalidPEMDocument` after the whole build. Nothing secret is printed: only which
# of those shapes the secret had and how long the parts are.
#
# Reads API_KEY_P8, API_KEY (the key id) and API_ISSUER (the issuer id) from the environment.
set -euo pipefail

out="${1:?the file to write the key to}"
raw="$(printf '%s' "${API_KEY_P8:?}" | tr -d '\r')"
pem_re='-----BEGIN [A-Z ]*PRIVATE KEY-----'

# (the key's base64, from between the BEGIN and END lines, wherever the line breaks went)
body_of() { sed -E -e 's/-----(BEGIN|END) [A-Z ]*PRIVATE KEY-----/ /g' | tr -d ' \n\t'; }

if printf '%s' "$raw" | grep -Eq -- "$pem_re"; then
  shape="the .p8 file"
  [ "$(printf '%s\n' "$raw" | wc -l | tr -d ' ')" -le 1 ] && shape="the .p8 file on one line"
  body="$(printf '%s' "$raw" | body_of)"
else
  decoded="$(printf '%s' "$raw" | tr -d ' \n\t' | base64 --decode 2>/dev/null | tr -d '\r' || true)"
  if printf '%s' "$decoded" | grep -Eq -- "$pem_re"; then
    shape="the .p8 file in base64"
    body="$(printf '%s' "$decoded" | body_of)"
  else
    shape="base64 without BEGIN/END lines"
    body="$(printf '%s' "$raw" | tr -d ' \n\t')"
  fi
fi

{
  echo '-----BEGIN PRIVATE KEY-----'
  printf '%s\n' "$body" | fold -w 64
  echo '-----END PRIVATE KEY-----'
} > "$out"

# (what the other two look like: a key id is 10 letters and digits, an issuer id a UUID)
key_ok=no; [[ "${API_KEY:-}" =~ ^[A-Z0-9]{10}$ ]] && key_ok=yes
issuer_ok=no; [[ "${API_ISSUER:-}" =~ ^[0-9a-fA-F]{8}-([0-9a-fA-F]{4}-){3}[0-9a-fA-F]{12}$ ]] && issuer_ok=yes
echo "APPLE_API_KEY_P8 read as $shape (${#raw} characters, key ${#body}); APPLE_API_KEY a key id: $key_ok (${#API_KEY} characters); APPLE_API_ISSUER a UUID: $issuer_ok"

if ! openssl pkey -in "$out" -noout 2>/dev/null; then
  hint=""
  [ "$key_ok" = no ] && [ "${#API_KEY}" -gt 100 ] && hint=" APPLE_API_KEY looks like the key itself: are APPLE_API_KEY and APPLE_API_KEY_P8 swapped?"
  echo "::error::APPLE_API_KEY_P8 is no App Store Connect API key (read as $shape, ${#body} characters of key; a .p8 has about 200). Paste the whole AuthKey_<key id>.p8 into the secret (docs/UPDATES.md).$hint"
  exit 1
fi
[ "$key_ok" = yes ] || echo "::warning::APPLE_API_KEY does not look like a key id (10 letters and digits, as in AuthKey_<key id>.p8)."
[ "$issuer_ok" = yes ] || echo "::warning::APPLE_API_ISSUER does not look like an issuer id (a UUID, App Store Connect › Users and Access › Integrations)."
