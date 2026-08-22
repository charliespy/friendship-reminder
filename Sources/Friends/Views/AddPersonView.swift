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

    private let frequencyOptions: [(label: String, days: Int)] = [
        ("Monthly", 30), ("Quarterly", 90), ("Semiannual", 180), ("Yearly", 365)
    ]

    var body: some View {
        Form {
            TextField("Name", text: $name)

            Picker("Frequency", selection: $frequencyDays) {
                ForEach(frequencyOptions, id: \.days) { option in
                    Text(option.label).tag(option.days)
                }
            }

            DatePicker("Last connected", selection: $lastConnectedDate, displayedComponents: .date)

            if let errorMessage {
                Text(errorMessage).foregroundStyle(.red).font(.caption)
            }

            Button("Add Person") {
                Task { await save() }
            }
            .disabled(name.trimmingCharacters(in: .whitespaces).isEmpty || isSaving)
        }
        .padding()
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
