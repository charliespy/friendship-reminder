// Sources/Friends/Views/BlockedDatesView.swift
import SwiftUI
import SwiftData

struct BlockedDatesView: View {
    @Environment(\.modelContext) private var modelContext
    @ObservedObject private var settings = AppSettings.shared
    @Query private var blockedRanges: [BlockedDateRange]
    @Query private var people: [Person]

    @State private var label: String = ""
    @State private var startDate: Date = Date()
    @State private var endDate: Date = Date()
    @State private var isSaving = false
    @State private var errorMessage: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            VStack(alignment: .leading, spacing: 2) {
                Text("Blocked Dates").font(FriendsTheme.sectionTitle).foregroundStyle(FriendsTheme.ink)
                Text("Trips or busy stretches where reminders shouldn't land — everyone reschedules the moment you add one.")
                    .font(FriendsTheme.body(12))
                    .foregroundStyle(FriendsTheme.inkSoft)
            }

            VStack(alignment: .leading, spacing: 16) {
                VStack(alignment: .leading, spacing: 6) {
                    FieldLabel(text: "Label")
                    TextField("e.g. Hawaii trip", text: $label)
                        .textFieldStyle(FriendsTextFieldStyle())
                }

                HStack(spacing: 20) {
                    VStack(alignment: .leading, spacing: 6) {
                        FieldLabel(text: "Start")
                        DatePicker("", selection: $startDate, displayedComponents: .date).labelsHidden()
                    }
                    VStack(alignment: .leading, spacing: 6) {
                        FieldLabel(text: "End")
                        DatePicker("", selection: $endDate, displayedComponents: .date).labelsHidden()
                    }
                }

                if let errorMessage {
                    HStack(alignment: .top, spacing: 6) {
                        Image(systemName: "exclamationmark.triangle.fill").font(.system(size: 11))
                        Text(errorMessage).font(FriendsTheme.body(12))
                    }
                    .foregroundStyle(FriendsTheme.rust)
                }

                Button("Add Blocked Range") {
                    Task { await addRange() }
                }
                .buttonStyle(PrimaryButtonStyle(isEnabled: !label.trimmingCharacters(in: .whitespaces).isEmpty && !isSaving))
                .disabled(label.trimmingCharacters(in: .whitespaces).isEmpty || isSaving)
            }
            .friendsCard()

            VStack(alignment: .leading, spacing: 8) {
                SectionLabel(text: "Upcoming Blocks")
                if blockedRanges.isEmpty {
                    Text("No blocked dates yet.")
                        .font(FriendsTheme.body(12))
                        .foregroundStyle(FriendsTheme.inkFaint)
                } else {
                    VStack(spacing: 6) {
                        ForEach(blockedRanges) { range in
                            HStack(spacing: 10) {
                                Circle().fill(FriendsTheme.amber).frame(width: 6, height: 6)
                                Text(range.label)
                                    .font(FriendsTheme.body(12, weight: .semibold))
                                    .foregroundStyle(FriendsTheme.ink)
                                Spacer()
                                Text("\(range.startDate.formatted(date: .abbreviated, time: .omitted)) – \(range.endDate.formatted(date: .abbreviated, time: .omitted))")
                                    .font(FriendsTheme.body(11))
                                    .foregroundStyle(FriendsTheme.inkSoft)
                            }
                            .friendsCard(padding: 9)
                        }
                    }
                }
            }

            Spacer()
        }
    }

    private func addRange() async {
        isSaving = true
        defer { isSaving = false }

        let range = BlockedDateRange(startDate: startDate, endDate: endDate, label: label)
        modelContext.insert(range)

        do {
            // Per spec: adding a range immediately reschedules everyone against
            // the complete, current set of blocked ranges (including this new one).
            let allRanges = blockedRanges + [range]
            let coordinator = SyncCoordinator()
            try await coordinator.runSyncCycle(people: people, blockedRanges: allRanges, settings: settings)
            label = ""
            errorMessage = nil
        } catch {
            errorMessage = "Could not reschedule people: \(error.localizedDescription)"
        }
    }
}
