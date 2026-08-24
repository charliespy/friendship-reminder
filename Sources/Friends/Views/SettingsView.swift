// Sources/Friends/Views/SettingsView.swift
import SwiftUI
import SwiftData

struct SettingsView: View {
    @Environment(\.modelContext) private var modelContext
    @ObservedObject private var settings = AppSettings.shared
    @Query private var people: [Person]
    @Query private var blockedRanges: [BlockedDateRange]

    @State private var isResyncing = false
    @State private var resyncMessage: String?
    @State private var resyncIsError = false
    @State private var showResyncConfirmation = false

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            VStack(alignment: .leading, spacing: 2) {
                Text("Settings").font(FriendsTheme.sectionTitle).foregroundStyle(FriendsTheme.ink)
                Text("Choose which days are fair game for scheduling meetups.")
                    .font(FriendsTheme.body(12))
                    .foregroundStyle(FriendsTheme.inkSoft)
            }

            VStack(alignment: .leading, spacing: 10) {
                FieldLabel(text: "Meeting Days")
                VStack(alignment: .leading, spacing: 8) {
                    ForEach(DayToggle.allCases) { toggle in
                        toggleRow(toggle)
                    }
                }
            }
            .friendsCard()

            VStack(alignment: .leading, spacing: 10) {
                FieldLabel(text: "Maintenance")
                VStack(alignment: .leading, spacing: 10) {
                    Text("Wipe every reminder in the Friends list and recreate one fresh reminder per person, based on today's date.")
                        .font(FriendsTheme.body(12))
                        .foregroundStyle(FriendsTheme.inkSoft)

                    if let resyncMessage {
                        HStack(alignment: .top, spacing: 6) {
                            Image(systemName: resyncIsError ? "exclamationmark.triangle.fill" : "checkmark.circle.fill")
                                .font(.system(size: 11))
                            Text(resyncMessage).font(FriendsTheme.body(12))
                        }
                        .foregroundStyle(resyncIsError ? FriendsTheme.rust : FriendsTheme.sage)
                    }

                    Button {
                        showResyncConfirmation = true
                    } label: {
                        HStack(spacing: 6) {
                            if isResyncing {
                                ProgressView().controlSize(.small)
                            } else {
                                Image(systemName: "arrow.triangle.2.circlepath")
                            }
                            Text("Resync All Reminders")
                        }
                    }
                    .buttonStyle(PrimaryButtonStyle(tint: FriendsTheme.clay, isEnabled: !isResyncing))
                    .disabled(isResyncing)
                    .confirmationDialog(
                        "Delete and recreate every reminder?",
                        isPresented: $showResyncConfirmation,
                        titleVisibility: .visible
                    ) {
                        Button("Resync All Reminders", role: .destructive) {
                            Task { await resyncAll() }
                        }
                    } message: {
                        Text("This removes all current reminders in the Friends list and rebuilds them from today's schedules.")
                    }
                }
            }
            .friendsCard()

            Spacer()
        }
    }

    private func toggleRow(_ toggle: DayToggle) -> some View {
        let isSelected = settings.dayToggle == toggle
        return Button {
            settings.dayToggle = toggle
        } label: {
            HStack(spacing: 10) {
                Image(systemName: isSelected ? "largecircle.fill.circle" : "circle")
                    .foregroundStyle(isSelected ? FriendsTheme.rust : FriendsTheme.inkFaint)
                Text(toggle.displayName)
                    .font(FriendsTheme.body(13, weight: isSelected ? .semibold : .regular))
                    .foregroundStyle(FriendsTheme.ink)
                Spacer()
            }
            .padding(.vertical, 4)
        }
        .buttonStyle(.plain)
    }

    private func resyncAll() async {
        isResyncing = true
        defer { isResyncing = false }

        do {
            let coordinator = SyncCoordinator()
            try await coordinator.resyncAll(people: people, blockedRanges: blockedRanges, settings: settings)
            resyncIsError = false
            resyncMessage = "Resynced \(people.count) reminder\(people.count == 1 ? "" : "s")."
        } catch {
            resyncIsError = true
            resyncMessage = "Could not resync: \(error.localizedDescription)"
        }
    }
}
