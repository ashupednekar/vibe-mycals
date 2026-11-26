# MyCals iOS widget — free via Scriptable

Native WidgetKit widgets need a paid Apple Developer account (App Groups).
Scriptable gives you a free home-screen widget without any Apple dev account.

1. Install **Scriptable** from the App Store.
2. In MyCals: **You → Generate widget token** (regenerating invalidates the old one).
3. Scriptable → **+** → paste `MyCalsWidget.js` → replace `PASTE_WIDGET_TOKEN_HERE` with the token.
4. Long-press the Home Screen → **Add Widget** → **Scriptable** → choose this script.

The widget shows today's kcal / target, turns red at 80%, and refreshes every 30 minutes.
The token is read-only and only exposes the calorie summary.
