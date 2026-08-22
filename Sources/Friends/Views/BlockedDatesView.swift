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
        VStack(alignment: .leading) {
            Form {
                TextField("Label", text: $label)
                DatePicker("Start", selection: $startDate, displayedComponents: .date)
                DatePicker("End", selection: $endDate, displayedComponents: .date)

                if let errorMessage {
                    Text(errorMessage).foregroundStyle(.red).font(.caption)
                }

                Button("Add Blocked Range") {
                    Task { await addRange() }
                }
                .disabled(label.trimmingCharacters(in: .whitespaces).isEmpty || isSaving)
            }

            Divider()

            List(blockedRanges) { range in
                Text("\(range.label): \(range.startDate.formatted(date: .abbreviated, time: .omitted)) – \(range.endDate.formatted(date: .abbreviated, time: .omitted))")
            }
        }
        .padding()
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
