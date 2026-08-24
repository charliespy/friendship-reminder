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
        .windowResizability(.contentSize)
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
