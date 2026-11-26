import React from "react";
import * as SecureStore from "expo-secure-store";
import type { WidgetTaskHandlerProps } from "react-native-android-widget";
import { CalorieWidget } from "./CalorieWidget";

const API = "https://mycals.ashupednekar49.workers.dev";
const SESSION_KEY = "mycals.session";

const today = () => {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
};

export async function widgetTaskHandler({ widgetAction, renderWidget }: WidgetTaskHandlerProps) {
  if (widgetAction === "WIDGET_DELETED") return;

  let kcal = 0;
  let target = 1200;
  try {
    const token = await SecureStore.getItemAsync(SESSION_KEY);
    if (token) {
      const res = await fetch(`${API}/api/diary?date=${today()}`, {
        headers: { authorization: `Bearer ${token}` },
      });
      if (res.ok) {
        const data = await res.json();
        kcal = Math.round(data?.totals?.kcal ?? 0);
        target = Math.round(data?.targets?.calorie ?? 1200);
      }
    }
  } catch {}

  renderWidget(<CalorieWidget kcal={kcal} target={target} />);
}
