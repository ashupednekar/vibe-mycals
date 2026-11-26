#!/usr/bin/env bash
# Build and deploy the Worker API, the Dioxus app, and the Cloudflare Pages site.
set -euo pipefail
cd "$(dirname "$0")"

# ---- local secrets -------------------------------------------------------
touch .secrets.local
chmod 600 .secrets.local
# Optional .dev.vars from `npx wrangler dev`; then the persisted deploy secrets.
if [ -f .dev.vars ]; then set -a; . ./.dev.vars; set +a; fi
set -a; . ./.secrets.local; set +a

if [ -z "${SESSION_SECRET:-}" ]; then
  SESSION_SECRET="$(openssl rand -hex 32)"
  printf 'SESSION_SECRET=%s\n' "$SESSION_SECRET" >> .secrets.local
fi

# ---- web build -----------------------------------------------------------
echo "▸ building Dioxus app"
rm -rf target/dx
(cd app && dx build --platform web --release)
rm -rf app/dist
mkdir -p app/dist
PUBLIC_DIR="$(find target/dx -path '*/release/web/public' -type d | head -1)"
[ -n "$PUBLIC_DIR" ] || { echo "web build output not found"; exit 1; }
cp -R "$PUBLIC_DIR"/. app/dist/
echo "▸ web assets in app/dist"

# ---- database ------------------------------------------------------------
echo "▸ applying D1 migrations"
printf 'y\n' | wrangler d1 migrations apply mycals --remote

# ---- worker secrets ------------------------------------------------------
put_secret() {
  local name="$1" value="${2:-}"
  if [ -n "$value" ]; then
    printf '%s' "$value" | wrangler secret put "$name" >/dev/null
    echo "  ✓ $name"
  else
    echo "  – $name not set (skipped)"
  fi
}
echo "▸ syncing Worker secrets"
put_secret SESSION_SECRET "$SESSION_SECRET"
put_secret GEMINI_API_KEY "${GEMINI_API_KEY:-}"
put_secret GOOGLE_CLIENT_ID "${GOOGLE_CLIENT_ID:-}"
put_secret GOOGLE_CLIENT_SECRET "${GOOGLE_CLIENT_SECRET:-}"

# ---- worker + site -------------------------------------------------------
echo "▸ deploying Worker"
DEPLOY_LOG="$(mktemp)"
wrangler deploy 2>&1 | tee "$DEPLOY_LOG"
APP_URL="$(grep -Eo 'https://[A-Za-z0-9.-]+\.workers\.dev' "$DEPLOY_LOG" | head -1 || true)"
if [ -z "$APP_URL" ]; then
  APP_URL="https://mycals.ashupednekar49.workers.dev"
fi

echo "▸ deploying landing page to Cloudflare Pages"
SITE_DIR="$(mktemp -d)"
cp -R site/. "$SITE_DIR/"
python3 - "$SITE_DIR/index.html" "$APP_URL" <<'PY'
import sys
path, url = sys.argv[1], sys.argv[2]
html = open(path).read().replace("__APP_URL__", url)
open(path, "w").write(html)
PY
wrangler pages project create mycals --production-branch=main >/dev/null 2>&1 || true
wrangler pages deploy "$SITE_DIR" --project-name=mycals --branch=main --commit-dirty=true

echo
echo "✓ live"
echo "  app:     $APP_URL"
echo "  landing: https://mycals.pages.dev"
echo
echo "Next, optional: add GOOGLE_CLIENT_ID / GOOGLE_CLIENT_SECRET / GEMINI_API_KEY"
echo "to .secrets.local, then rerun ./deploy.sh to enable Google sign-in and scanning."
