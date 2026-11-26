// MyCals — free home-screen widget for Scriptable (iOS).
// 1) MyCals app → You → Generate widget token, copy it.
// 2) Scriptable → + → paste this file, replace TOKEN below.
// 3) Long-press Home Screen → Add Widget → Scriptable → choose this script.
const API = "https://mycals.ashupednekar49.workers.dev";
const TOKEN = "PASTE_WIDGET_TOKEN_HERE";

const now = new Date();
const p = (n) => String(n).padStart(2, "0");
const date = `${now.getFullYear()}-${p(now.getMonth() + 1)}-${p(now.getDate())}`;

const widget = new ListWidget();
widget.backgroundColor = new Color("#0b1210");
widget.setPadding(14, 14, 14, 14);
widget.refreshAfterDate = new Date(Date.now() + 30 * 60 * 1000);

async function build() {
  const req = new Request(`${API}/api/widget/summary?token=${encodeURIComponent(TOKEN)}&date=${date}`);
  req.timeoutInterval = 10;
  const d = await req.loadJSON();
  if (d.error) throw new Error(d.error);

  const title = widget.addText("MyCals");
  title.font = Font.boldSystemFont(12);
  title.textColor = new Color("#7bd88f");

  widget.addSpacer(2);
  const warning = d.kcal >= d.target * 0.8;
  const value = widget.addText(`${Math.round(d.kcal)} / ${d.target}`);
  value.font = Font.boldSystemFont(28);
  value.textColor = warning ? new Color("#ff8a65") : new Color("#edf3ea");

  const sub = widget.addText(warning ? "80%+ of today's target" : "kcal today");
  sub.font = Font.systemFont(11);
  sub.textColor = new Color("#95a39a");

  widget.addSpacer(8);
  const ctx = new DrawContext();
  ctx.size = new Size(150, 6);
  ctx.opaque = false;
  ctx.setFillColor(new Color("#24322a"));
  ctx.fillRect(new Rect(0, 0, 150, 6));
  const pct = Math.max(0, Math.min(1, d.kcal / Math.max(1, d.target)));
  ctx.setFillColor(warning ? new Color("#ff8a65") : new Color("#7bd88f"));
  ctx.fillRect(new Rect(0, 0, 150 * pct, 6));
  const img = widget.addImage(ctx.getImage());
  img.imageSize = new Size(150, 6);

  widget.addSpacer(4);
  const foot = widget.addText(`${d.protein} g protein`);
  foot.font = Font.systemFont(10);
  foot.textColor = new Color("#6f7f73");
}

try {
  await build();
} catch (e) {
  const title = widget.addText("MyCals");
  title.font = Font.boldSystemFont(13);
  title.textColor = new Color("#ff8a65");
  const msg = widget.addText(TOKEN.startsWith("PASTE") ? "Add your widget token in Scriptable" : "Open MyCals to refresh");
  msg.font = Font.systemFont(11);
  msg.textColor = new Color("#edf3ea");
}

Script.setWidget(widget);
Script.complete();
