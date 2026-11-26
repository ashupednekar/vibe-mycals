// MyCals WidgetKit extension (scaffold).
//
// Requires a paid Apple Developer account and an App Group shared by the app
// and this extension: group.app.mycals.mobile
//
// The app must write "kcal" and "target" into UserDefaults(suiteName:) after
// each diary refresh. Free Apple ID provisioning does not grant App Groups.
import WidgetKit
import SwiftUI

private let appGroup = "group.app.mycals.mobile"

struct CalorieEntry: TimelineEntry {
    let date: Date
    let kcal: Int
    let target: Int
}

struct CalorieProvider: TimelineProvider {
    func placeholder(in context: Context) -> CalorieEntry {
        CalorieEntry(date: Date(), kcal: 720, target: 1200)
    }
    func getSnapshot(in context: Context, completion: @escaping (CalorieEntry) -> Void) {
        completion(read())
    }
    func getTimeline(in context: Context, completion: @escaping (Timeline<CalorieEntry>) -> Void) {
        let next = Calendar.current.date(byAdding: .minute, value: 30, to: Date())!
        completion(Timeline(entries: [read()], policy: .after(next)))
    }
    private func read() -> CalorieEntry {
        let defaults = UserDefaults(suiteName: appGroup)
        let kcal = defaults?.integer(forKey: "kcal") ?? 0
        let target = max(1, defaults?.integer(forKey: "target") ?? 1200)
        return CalorieEntry(date: Date(), kcal: kcal, target: target)
    }
}

struct MyCalsWidgetEntryView: View {
    var entry: CalorieEntry
    private var progress: Double { min(1, Double(entry.kcal) / Double(max(1, entry.target))) }
    private var warning: Bool { progress >= 0.8 }
    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("MyCals")
                .font(.caption.weight(.bold))
                .foregroundStyle(Color(red: 0.48, green: 0.85, blue: 0.56))
            Text("\(entry.kcal) / \(entry.target)")
                .font(.system(size: 28, weight: .bold, design: .rounded))
                .foregroundStyle(warning ? Color(red: 1, green: 0.54, blue: 0.4) : .white)
            Text(warning ? "80%+ of target" : "kcal today")
                .font(.caption2)
                .foregroundStyle(.secondary)
            ProgressView(value: progress)
                .tint(warning ? Color(red: 1, green: 0.54, blue: 0.4) : Color(red: 0.48, green: 0.85, blue: 0.56))
        }
        .containerBackground(for: .widget) {
            Color(red: 0.043, green: 0.07, blue: 0.063)
        }
    }
}

@main
struct MyCalsWidget: Widget {
    let kind = "MyCalsWidget"
    var body: some WidgetConfiguration {
        StaticConfiguration(kind: kind, provider: CalorieProvider()) { entry in
            MyCalsWidgetEntryView(entry: entry)
        }
        .configurationDisplayName("MyCals calories")
        .description("Today's calories at a glance.")
        .supportedFamilies([.systemSmall, .systemMedium])
    }
}
