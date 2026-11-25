# MyCals

A deliberately small calorie tracker with **Google sign-in**, **family sharing**, and **photo → calories** scanning with Gemini. Rust on Cloudflare Workers, Dioxus on the web, Expo for the native apps.

```
api/                  Cloudflare Worker (Rust)
  cmd/worker/         Worker entrypoint (#[event(fetch)])
  internal/server/    auth, diary, family, foods, Gemini scan
app/                  Dioxus web client
common/               shared types, nutrition targets, session tokens
site/                 marketing site → Cloudflare Pages
mobile/               Expo app (Android + iOS)
migrations/           D1 schema + seed foods
deploy.sh             builds + deploys everything
```

## What it does

- **Google sign-in** — OAuth authorization-code flow. The Worker verifies the code with Google, upserts the user in D1 and issues an HMAC-signed session cookie. The Expo client uses the same flow with a `mycals://` deep link and Bearer tokens.
- **Family calories** — create a family, share a six-character invite code, join. Members see each other's calories and meals for the day if sharing is on. Up to 8 people; ownership transfers on leave.
- **Calorie diary** — breakfast / lunch / dinner / snacks, 60+ seeded foods, custom foods, per-100 g portions, 7-day calorie chart.
- **Scan with Gemini** — photo goes to the Worker, the Worker calls Gemini with a structured nutrition schema, and the client logs the detected items. The API key is a Worker secret; it never reaches the browser.
- **Demo mode** — `DEMO_MODE=1` enables a one-click demo family (`DEMO42`) so the app is usable before Google credentials are configured.

## Deploy

```bash
./deploy.sh
```

It builds the Dioxus app, applies D1 migrations, syncs Worker secrets, deploys the Worker + static assets, and publishes `site/` to Cloudflare Pages. Requires a logged-in `wrangler`.

Set optional secrets in `.secrets.local` (gitignored) or your shell:

```bash
SESSION_SECRET=...        # auto-generated if absent
GEMINI_API_KEY=...        # enables photo scanning
GOOGLE_CLIENT_ID=...      # enables Google sign-in
GOOGLE_CLIENT_SECRET=...
```

D1: `mycals` (`wrangler d1 create mycals` if you fork this).

## Local development

```bash
# terminal 1 — worker + SPA assets (same origin, cookies work)
npx wrangler dev
# → http://localhost:8787

# terminal 2 — Dioxus hot reload (optional; proxy /api to :8787)
cd app && dx serve --platform web
```

Create `.dev.vars` for local secrets:

```bash
SESSION_SECRET=dev-secret-change-me
DEMO_MODE=1
GEMINI_API_KEY=
GOOGLE_CLIENT_ID=
GOOGLE_CLIENT_SECRET=
```

Apply migrations locally: `wrangler d1 migrations apply mycals --local`.

## Mobile (Expo)

```bash
cd mobile
npm install
npx expo start
```

- **EAS publish** — `.github/workflows/mobile.yml` runs `eas build` (and optional `--auto-submit`) for Android/iOS.
- **Sideloadable APK** — `.github/workflows/android-apk.yml` builds with Expo Prebuild + Gradle and uploads `vibe-mycals.apk` to the `mobile-latest` release. The landing page links there.
- Set `EXPO_PUBLIC_API_URL` in the build environment to the deployed Worker URL (defaults to `http://localhost:8787` in dev).

## API map

| Method | Path | Purpose |
| --- | --- | --- |
| GET | `/api/auth/google/start` | begin Google OAuth (`?mobile=1` for the app) |
| GET | `/api/auth/google/callback` | exchange code, set session / deep-link token |
| POST | `/api/dev/login` | demo user (DEMO_MODE only) |
| GET | `/api/me` | current user |
| PUT | `/api/profile` | profile + auto calorie target |
| GET | `/api/diary?date=` | day view |
| POST | `/api/diary/entries` | log a food or scan item |
| PATCH/DELETE | `/api/diary/entries/:id` | adjust or remove |
| GET | `/api/foods?q=` | public + personal + family foods |
| GET | `/api/foods/barcode/:code` | D1, then Open Food Facts |
| POST | `/api/scan` | Gemini vision → nutrition |
| GET | `/api/family?date=` | household + member calories |
| POST | `/api/family` | create |
| POST | `/api/family/join` | join by code |
| POST | `/api/family/leave` | leave (transfers ownership) |
| POST | `/api/family/rotate` | rotate invite code |
| GET | `/api/family/members/:id/diary` | member day (respects sharing) |

## Notes

- Calories only in the UI — macros exist in the schema but aren't surfaced.
- Session tokens are HMAC-SHA256, 30 days, `HttpOnly; SameSite=Lax; Secure` on https.
- Google OAuth in the native app intentionally uses the system browser; embedded webviews are blocked by Google.
- Built with `worker 0.9`, `dioxus 0.7`, D1, Workers Assets, and Cloudflare Pages.

## GitHub Actions

`.github/workflows/deploy.yml` builds and deploys the Worker, the Dioxus app and the Pages site
on every push/merge to `main` (and on manual dispatch). Add these repository secrets:

| Secret | Purpose |
| --- | --- |
| `CLOUDFLARE_API_TOKEN` | Worker + D1 + Pages deploy token (Workers Scripts:Edit, D1:Edit, Pages:Edit) |
| `CLOUDFLARE_ACCOUNT_ID` | Cloudflare account id |
| `SESSION_SECRET` | HMAC key for session tokens |
| `GEMINI_API_KEY` | photo scanning |
| `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET` | Google sign-in |
| `EXPO_TOKEN` | EAS build/submit (mobile workflow) |

`android-apk.yml` builds a sideloadable APK with Expo Prebuild + Gradle and publishes it to the
`mobile-latest` release. `mobile.yml` runs EAS builds and optional store submission. Run
`npx eas init` once in `mobile/` to attach an Expo project before using EAS.
