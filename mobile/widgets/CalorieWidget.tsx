import React from "react";
import { FlexWidget, TextWidget } from "react-native-android-widget";

export function CalorieWidget({ kcal, target }: { kcal: number; target: number }) {
  const pct = Math.min(1, kcal / Math.max(1, target));
  const warning = pct >= 0.8;
  return (
    <FlexWidget
      clickAction="OPEN_URI"
      clickActionData={{ uri: "mycals://scan" }}
      accessibilityLabel="Open MyCals scanner"
      style={{
        width: "match_parent",
        height: "match_parent",
        backgroundColor: "#0b1210",
        borderRadius: 24,
        padding: 18,
        flexDirection: "column",
        justifyContent: "center",
      }}
    >
      <TextWidget
        text="MyCals"
        style={{ fontSize: 12, color: "#7bd88f", fontWeight: "bold", letterSpacing: 1 }}
      />
      <TextWidget
        text={`${kcal} / ${target}`}
        style={{ fontSize: 30, color: warning ? "#ff8a65" : "#edf3ea", fontWeight: "bold" }}
      />
      <TextWidget
        text={warning ? "80%+ of today's target" : "kcal today"}
        style={{ fontSize: 12, color: "#95a39a" }}
      />
      <FlexWidget
        style={{
          width: "match_parent",
          height: 6,
          marginTop: 10,
          borderRadius: 3,
          backgroundColor: "#24322a",
          flexDirection: "row",
        }}
      >
        {pct > 0 && (
          <FlexWidget
            style={{
              flex: pct,
              height: 6,
              borderRadius: 3,
              backgroundColor: warning ? "#ff8a65" : "#7bd88f",
            }}
          />
        )}
        {pct < 1 && (
          <FlexWidget style={{ flex: 1 - pct, height: 6, backgroundColor: "#24322a" }} />
        )}
      </FlexWidget>
    </FlexWidget>
  );
}
