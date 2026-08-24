import SwiftUI
import SwiftData
import AppKit

@main
struct FriendsApp: App {
    var sharedModelContainer: ModelContainer = {
        let schema = Schema([Person.self, BlockedDateRange.self])
        let config = ModelConfiguration(schema: schema, isStoredInMemoryOnly: false)
        do {
            return try ModelContainer(for: schema, configurations: [config])
        } catch {
            fatalError("Could not create ModelContainer: \(error)")
        }
    }()

    @State private var syncTimer: Timer?

    var body: some Scene {
        MenuBarExtra("Friends", systemImage: "person.2.fill") {
            MenuBarDropdownView()
                .task { await runSync() }
        }
        .modelContainer(sharedModelContainer)

        Settings {
            PreferencesView()
        }
        .modelContainer(sharedModelContainer)
        .windowResizability(.contentMinSize)
    }

    init() {
        // FriendsApp is a struct (required by the `App` protocol), so inside `init()`
        // `self` is an inout parameter and cannot be captured by the escaping closures
        // below (Swift error: "escaping closure captures mutating 'self' parameter").
        // Capture the container value itself instead and route through a static helper.
        let container = sharedModelContainer
        NotificationCenter.default.addObserver(
            forName: NSWorkspace.didWakeNotification,
            object: nil,
            queue: .main
        ) { _ in
            Task { await FriendsApp.runSync(using: container) }
        }
        Timer.scheduledTimer(withTimeInterval: 3600, repeats: true) { _ in
            Task { await FriendsApp.runSync(using: container) }
        }

        // Since this is a menu-bar-only (LSUIElement) app with no Dock icon,
        // launching it produces no visible window by default — the user just
        // sees a new menu bar icon. Open Preferences automatically every
        // launch so there's an obvious, immediate sign the app started.
        // `showSettingsWindow:` is the selector SwiftUI registers on
        // NSApplication for a `Settings` scene; there's no public API to
        // open it directly from outside a View's environment. Waiting for
        // didFinishLaunchingNotification (rather than firing immediately
        // from init, which runs before AppKit/NSApplication has finished
        // standing up — accessing NSApp itself too early can crash) is what
        // makes this reliable.
        NotificationCenter.default.addObserver(
            forName: NSApplication.didFinishLaunchingNotification,
            object: nil,
            queue: .main
        ) { _ in
            FriendsApp.openPreferences()
        }
    }

    private static func openPreferences() {
        NSApplication.shared.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil)
        NSApplication.shared.activate(ignoringOtherApps: true)
    }

    @MainActor
    private func runSync() async {
        await FriendsApp.runSync(using: sharedModelContainer)
    }

    @MainActor
    private static func runSync(using container: ModelContainer) async {
        let context = container.mainContext
        let people = (try? context.fetch(FetchDescriptor<Person>())) ?? []
        let ranges = (try? context.fetch(FetchDescriptor<BlockedDateRange>())) ?? []
        let coordinator = SyncCoordinator()
        try? await coordinator.runSyncCycle(people: people, blockedRanges: ranges, settings: AppSettings.shared)
    }
}
