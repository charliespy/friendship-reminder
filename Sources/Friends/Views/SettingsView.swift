// Sources/Friends/Views/SettingsView.swift
import SwiftUI
import SwiftData

struct SettingsView: View {
    @Environment(\.modelContext) private var modelContext
    @ObservedObject private var settings = AppSettings.shared
    @Query private var people: [Person]
    @Query private var blockedRanges: [BlockedDateRange]

    @State private var isDeletingAll = false
    @State private var isCreatingAll = false
    @State private var maintenanceMessage: String?
    @State private var maintenanceIsError = false
    @State private var showDeleteAllConfirmation = false

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
                    Text("Manually delete every reminder in the Friends list, or (re)create one for everyone based on today's date.")
                        .font(FriendsTheme.body(12))
                        .foregroundStyle(FriendsTheme.inkSoft)

                    if let maintenanceMessage {
                        HStack(alignment: .top, spacing: 6) {
                            Image(systemName: maintenanceIsError ? "exclamationmark.triangle.fill" : "checkmark.circle.fill")
                                .font(.system(size: 11))
                            Text(maintenanceMessage).font(FriendsTheme.body(12))
                        }
                        .foregroundStyle(maintenanceIsError ? FriendsTheme.rust : FriendsTheme.sage)
                    }

                    HStack(spacing: 10) {
                        Button {
                            showDeleteAllConfirmation = true
                        } label: {
                            HStack(spacing: 6) {
                                if isDeletingAll {
                                    ProgressView().controlSize(.small)
                                } else {
                                    Image(systemName: "trash")
                                }
                                Text("Delete All Reminders")
                            }
                        }
                        .buttonStyle(PrimaryButtonStyle(tint: FriendsTheme.rust, isEnabled: !isDeletingAll && !isCreatingAll))
                        .disabled(isDeletingAll || isCreatingAll)
                        .confirmationDialog(
                            "Delete every reminder in the Friends list?",
                            isPresented: $showDeleteAllConfirmation,
                            titleVisibility: .visible
                        ) {
                            Button("Delete All Reminders", role: .destructive) {
                                Task { await deleteAll() }
                            }
                        } message: {
                            Text("People stay in Friends, but their reminders are removed until you add them again.")
                        }

                        Button {
                            Task { await createAll() }
                        } label: {
                            HStack(spacing: 6) {
                                if isCreatingAll {
                                    ProgressView().controlSize(.small)
                                } else {
                                    Image(systemName: "plus")
                                }
                                Text("Add Reminders for Today")
                            }
                        }
                        .buttonStyle(PrimaryButtonStyle(tint: FriendsTheme.clay, isEnabled: !isDeletingAll && !isCreatingAll))
                        .disabled(isDeletingAll || isCreatingAll)
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

    private func deleteAll() async {
        isDeletingAll = true
        defer { isDeletingAll = false }

        do {
            let coordinator = SyncCoordinator()
            try await coordinator.deleteAllReminders(people: people)
            maintenanceIsError = false
            maintenanceMessage = "Deleted all reminders."
        } catch {
            maintenanceIsError = true
            maintenanceMessage = "Could not delete reminders: \(error.localizedDescription)"
        }
    }

    private func createAll() async {
        isCreatingAll = true
        defer { isCreatingAll = false }

        do {
            let coordinator = SyncCoordinator()
            try await coordinator.createAllReminders(people: people, blockedRanges: blockedRanges, settings: settings)
            maintenanceIsError = false
            maintenanceMessage = "Added reminders for \(people.count) \(people.count == 1 ? "person" : "people")."
        } catch {
            maintenanceIsError = true
            maintenanceMessage = "Could not add reminders: \(error.localizedDescription)"
        }
    }
}
