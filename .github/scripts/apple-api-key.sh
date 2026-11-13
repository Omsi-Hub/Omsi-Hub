#!/usr/bin/env bash
# The App Store Connect API key for notarising the Mac app (docs/UPDATES.md "Signing the Mac app"),
# written to the file named in $1 as the PEM Tauri reads (APPLE_API_KEY_PATH), however the secret
# APPLE_API_KEY_P8 was pasted: the .p8 file as it is, the same on one line (GitHub's secret box or a
# copy can lose its line breaks), the key's base64 without its BEGIN/END lines, or the whole file in
# base64. Then checked, so a key that is no key stops here with what is wrong, not in Apple's
# notarisation (`invalidPEMDocument`, `invalidASN1Object`) after the whole build.
#
# A secret with more than one key in it (two .p8 files pasted one after the other, as up to 1.2.2):
# each is asked of Apple (`notarytool history`, on a Mac) and the one Apple takes for APPLE_API_KEY
# is used, with a warning to leave only that one in the secret.
#
# Nothing secret is printed: only the secret's shape, how many keys it holds and their lengths.
# Reads API_KEY_P8, API_KEY (the key id) and API_ISSUER (the issuer id) from the environment.
set -euo pipefail

out="${1:?the file to write the key to}"
raw="$(printf '%s' "${API_KEY_P8:?}" | tr -d '\r')"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
pem_re='-----BEGIN [A-Z ]*PRIVATE KEY-----'

# (the text that holds the key or keys: the secret, or the file it is the base64 of)
text="$raw"
if printf '%s' "$raw" | grep -Eq -- "$pem_re"; then
  shape="the .p8 file"
  [ "$(printf '%s\n' "$raw" | wc -l | tr -d ' ')" -le 1 ] && shape="the .p8 file on one line"
else
  decoded="$(printf '%s' "$raw" | tr -d ' \n\t' | base64 --decode 2>/dev/null | tr -d '\r' || true)"
  if printf '%s' "$decoded" | grep -Eq -- "$pem_re"; then
    shape="the .p8 file in base64"
    text="$decoded"
  else
    shape="base64 without BEGIN/END lines"
    text="-----BEGIN PRIVATE KEY----- $raw -----END PRIVATE KEY-----"
  fi
fi

# Every BEGIN … END block on its own, as one line of base64 each, wherever the line breaks went.
printf '%s' "$text" | tr '\n\t' '  ' \
  | perl -pe 's/-----(BEGIN|END) [A-Z ]*PRIVATE KEY-----/\n/g' \
  | tr -d ' ' | grep -v '^$' > "$work/blocks" || true

# Each block's DER, then each key in it: a block may hold two keys run together (a SEQUENCE after a
# SEQUENCE), as when two files were pasted and their BEGIN/END lines lost.
n=0
while IFS= read -r b64; do
  printf '%s' "$b64" | base64 --decode > "$work/block.der" 2>/dev/null || continue
  size=$(wc -c < "$work/block.der" | tr -d ' ')
  at=0
  while [ "$at" -lt "$size" ]; do
    dd if="$work/block.der" of="$work/rest.der" bs=1 skip="$at" 2>/dev/null
    len=$(openssl asn1parse -inform DER -in "$work/rest.der" 2>/dev/null | head -1 | sed -nE 's/.*hl= *([0-9]+) +l= *([0-9]+).*/\1 \2/p' | awk '{print $1+$2}' || true)
    [ -n "$len" ] && [ "$len" -gt 0 ] || break
    n=$((n + 1))
    dd if="$work/rest.der" of="$work/key$n.der" bs=1 count="$len" 2>/dev/null
    openssl pkey -inform DER -in "$work/key$n.der" -out "$work/key$n.pem" 2>/dev/null || rm -f "$work/key$n.pem"
    at=$((at + len))
  done
done < "$work/blocks"

keys=()
for ((i = 1; i <= n; i++)); do [ -f "$work/key$i.pem" ] && keys+=("$work/key$i.pem"); done

# (what the other two look like: a key id is 10 letters and digits, an issuer id a UUID)
key_ok=no; [[ "${API_KEY:-}" =~ ^[A-Z0-9]{10}$ ]] && key_ok=yes
issuer_ok=no; [[ "${API_ISSUER:-}" =~ ^[0-9a-fA-F]{8}-([0-9a-fA-F]{4}-){3}[0-9a-fA-F]{12}$ ]] && issuer_ok=yes
echo "APPLE_API_KEY_P8 read as $shape (${#raw} characters, ${#keys[@]} key(s) in it); APPLE_API_KEY a key id: $key_ok (${#API_KEY} characters); APPLE_API_ISSUER a UUID: $issuer_ok"
[ "$key_ok" = yes ] || echo "::warning::APPLE_API_KEY does not look like a key id (10 letters and digits, as in AuthKey_<key id>.p8)."
[ "$issuer_ok" = yes ] || echo "::warning::APPLE_API_ISSUER does not look like an issuer id (a UUID, App Store Connect › Users and Access › Integrations)."

if [ "${#keys[@]}" -eq 0 ]; then
  hint=""
  [ "$key_ok" = no ] && [ "${#API_KEY}" -gt 100 ] && hint=" APPLE_API_KEY looks like the key itself: are APPLE_API_KEY and APPLE_API_KEY_P8 swapped?"
  echo "::error::APPLE_API_KEY_P8 holds no App Store Connect API key (read as $shape). Paste the whole AuthKey_<key id>.p8 into the secret (docs/UPDATES.md).$hint"
  exit 1
fi

if [ "${#keys[@]}" -eq 1 ]; then
  cp "${keys[0]}" "$out"
  exit 0
fi

# More than one: the one Apple takes for this key id and issuer.
if ! command -v xcrun >/dev/null 2>&1; then
  echo "::error::APPLE_API_KEY_P8 holds ${#keys[@]} keys; leave only AuthKey_<key id>.p8 in it (docs/UPDATES.md)."
  exit 1
fi
for i in "${!keys[@]}"; do
  if xcrun notarytool history --key "${keys[$i]}" --key-id "$API_KEY" --issuer "$API_ISSUER" --output-format json > /dev/null 2>&1; then
    cp "${keys[$i]}" "$out"
    echo "::warning::APPLE_API_KEY_P8 holds ${#keys[@]} keys; Apple takes key $((i + 1)) for APPLE_API_KEY, which is used. Leave only that one in the secret (AuthKey_<key id>.p8, docs/UPDATES.md)."
    exit 0
  fi
done
echo "::error::APPLE_API_KEY_P8 holds ${#keys[@]} keys and Apple takes none of them for APPLE_API_KEY and APPLE_API_ISSUER: put the AuthKey_<key id>.p8 of that key id in the secret (docs/UPDATES.md)."
exit 1
