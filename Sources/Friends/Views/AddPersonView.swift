// Sources/Friends/Views/AddPersonView.swift
import SwiftUI
import SwiftData

struct AddPersonView: View {
    @Environment(\.modelContext) private var modelContext
    @ObservedObject private var settings = AppSettings.shared

    @State private var name: String = ""
    @State private var frequencyDays: Int = 30
    @State private var lastConnectedDate: Date = Date()
    @State private var isSaving = false
    @State private var errorMessage: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            VStack(alignment: .leading, spacing: 2) {
                Text("Add Person").font(FriendsTheme.sectionTitle).foregroundStyle(FriendsTheme.ink)
                Text("They'll get a reminder in your Friends list when it's time to reach out.")
                    .font(FriendsTheme.body(12))
                    .foregroundStyle(FriendsTheme.inkSoft)
            }

            VStack(alignment: .leading, spacing: 16) {
                VStack(alignment: .leading, spacing: 6) {
                    FieldLabel(text: "Name")
                    TextField("e.g. Jordan Lee", text: $name)
                        .textFieldStyle(FriendsTextFieldStyle())
                }

                VStack(alignment: .leading, spacing: 6) {
                    FieldLabel(text: "Frequency")
                    PillPicker(
                        options: FrequencyOption.all.map { ($0.label, $0.days) },
                        selection: $frequencyDays
                    )
                }

                VStack(alignment: .leading, spacing: 6) {
                    FieldLabel(text: "Last Connected")
                    DatePicker("", selection: $lastConnectedDate, displayedComponents: .date)
                        .labelsHidden()
                }
            }
            .friendsCard()

            if let errorMessage {
                HStack(alignment: .top, spacing: 6) {
                    Image(systemName: "exclamationmark.triangle.fill").font(.system(size: 11))
                    Text(errorMessage).font(FriendsTheme.body(12))
                }
                .foregroundStyle(FriendsTheme.rust)
            }

            Button("Add Person") {
                Task { await save() }
            }
            .buttonStyle(PrimaryButtonStyle(isEnabled: !name.trimmingCharacters(in: .whitespaces).isEmpty && !isSaving))
            .disabled(name.trimmingCharacters(in: .whitespaces).isEmpty || isSaving)

            Spacer()
        }
    }

    private func save() async {
        isSaving = true
        defer { isSaving = false }

        let blockedRanges = (try? modelContext.fetch(FetchDescriptor<BlockedDateRange>())) ?? []
        let blocked = blockedRanges.map { BlockedRange(start: $0.startDate, end: $0.endDate) }
        let result = SchedulingEngine.computeSchedule(
            lastConnectedDate: lastConnectedDate,
            frequencyDays: frequencyDays,
            toggle: settings.dayToggle,
            blockedRanges: blocked,
            calendar: .current
        )

        let person = Person(
            name: name,
            frequencyDays: frequencyDays,
            lastConnectedDate: lastConnectedDate,
            nextTargetDate: result.nextTargetDate,
            notificationDate: result.notificationDate
        )
        modelContext.insert(person)

        do {
            let coordinator = SyncCoordinator()
            try await coordinator.runSyncCycle(people: [person], blockedRanges: blockedRanges, settings: settings)
            name = ""
            errorMessage = nil
        } catch {
            errorMessage = "Could not create reminder: \(error.localizedDescription)"
        }
    }
}
