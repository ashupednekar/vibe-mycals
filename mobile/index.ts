import { registerRootComponent } from "expo";
import { Platform } from "react-native";

import App from "./App";

if (Platform.OS === "android") {
  // Android-only native module; guarded so the iOS bundle never loads it.
  const { registerWidgetTaskHandler } = require("react-native-android-widget");
  const { widgetTaskHandler } = require("./widgets/widgetTaskHandler");
  registerWidgetTaskHandler(widgetTaskHandler);
}

registerRootComponent(App);
