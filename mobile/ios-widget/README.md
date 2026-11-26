# MyCals iOS widget (WidgetKit)

This is a scaffold. Adding a WidgetKit extension **requires a paid Apple Developer
account**, because the app and widget must share an App Group
(`group.app.mycals.mobile`). Free Apple ID provisioning does not grant App Groups,
so it cannot be installed on the current free 7-day build.

To enable it:

1. In Xcode open `ios/MyCals.xcworkspace`.
2. File → New → Target → **Widget Extension** → name `MyCalsWidget`, uncheck "Include Configuration Intent".
3. Replace the generated widget Swift file with `ios-widget/MyCalsWidget.swift`.
4. Add the **App Groups** capability to both the `MyCals` and `MyCalsWidget` targets with `group.app.mycals.mobile`.
5. In the app, after loading the diary write the values:
   ```swift
   let d = UserDefaults(suiteName: "group.app.mycals.mobile")
   d?.set(Int(kcal), forKey: "kcal")
   d?.set(Int(target), forKey: "target")
   ```
   (or expose a small native module).
6. Build the app + widget target to your device.
