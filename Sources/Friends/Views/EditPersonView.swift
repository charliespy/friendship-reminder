// Sources/Friends/Views/EditPersonView.swift
// A modal sheet for editing an existing Person's name, frequency, and last
// connected date, or deleting them outright. Presented from PeopleListView.
import SwiftUI
import SwiftData

struct EditPersonView: View {
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss
    @ObservedObject private var settings = AppSettings.shared

    let person: Person

    @State private var name: String
    @State private var frequencyDays: Int
    @State private var lastConnectedDate: Date
    @State private var isSaving = false
    @State private var isDeleting = false
    @State private var errorMessage: String?
    @State private var showDeleteConfirmation = false

    init(person: Person) {
        self.person = person
        _name = State(initialValue: person.name)
        _frequencyDays = State(initialValue: person.frequencyDays)
        _lastConnectedDate = State(initialValue: person.lastConnectedDate)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    Text("Edit Person").font(FriendsTheme.sectionTitle).foregroundStyle(FriendsTheme.ink)
                    Text("Changes take effect the next time Friends syncs.")
                        .font(FriendsTheme.body(12))
                        .foregroundStyle(FriendsTheme.inkSoft)
                }
                Spacer()
                AvatarBadge(name: person.name, diameter: 34)
            }

            VStack(alignment: .leading, spacing: 16) {
                VStack(alignment: .leading, spacing: 6) {
                    FieldLabel(text: "Name")
                    TextField("Name", text: $name)
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

            HStack {
                Button(role: .destructive) {
                    showDeleteConfirmation = true
                } label: {
                    HStack(spacing: 5) {
                        Image(systemName: "trash")
                        Text("Delete")
                    }
                }
                .buttonStyle(QuietButtonStyle())
                .disabled(isSaving || isDeleting)
                .confirmationDialog(
                    "Remove \(person.name) from Friends?",
                    isPresented: $showDeleteConfirmation,
                    titleVisibility: .visible
                ) {
                    Button("Delete \(person.name)", role: .destructive) {
                        Task { await delete() }
                    }
                } message: {
                    Text("This also removes their reminder from the Friends list in Reminders.")
                }

                Spacer()

                Button("Cancel") { dismiss() }
                    .buttonStyle(QuietButtonStyle())
                    .disabled(isSaving || isDeleting)

                Button("Save Changes") {
                    Task { await save() }
                }
                .buttonStyle(PrimaryButtonStyle(isEnabled: !name.trimmingCharacters(in: .whitespaces).isEmpty && !isSaving && !isDeleting))
                .disabled(name.trimmingCharacters(in: .whitespaces).isEmpty || isSaving || isDeleting)
            }
        }
        .padding(20)
        .frame(width: 420)
        .background(FriendsTheme.cream)
    }

    private func save() async {
        isSaving = true
        defer { isSaving = false }

        person.name = name
        person.frequencyDays = frequencyDays
        person.lastConnectedDate = lastConnectedDate

        let blockedRanges = (try? modelContext.fetch(FetchDescriptor<BlockedDateRange>())) ?? []
        let blocked = blockedRanges.map { BlockedRange(start: $0.startDate, end: $0.endDate) }
        let result = SchedulingEngine.computeSchedule(
            lastConnectedDate: lastConnectedDate,
            frequencyDays: frequencyDays,
            toggle: settings.dayToggle,
            blockedRanges: blocked,
            calendar: .current
        )
        person.nextTargetDate = result.nextTargetDate
        person.notificationDate = result.notificationDate

        do {
            let coordinator = SyncCoordinator()
            try await coordinator.runSyncCycle(people: [person], blockedRanges: blockedRanges, settings: settings)
            dismiss()
        } catch {
            errorMessage = "Could not update reminder: \(error.localizedDescription)"
        }
    }

    private func delete() async {
        isDeleting = true
        defer { isDeleting = false }

        do {
            let coordinator = SyncCoordinator()
            try await coordinator.deletePerson(person)
            modelContext.delete(person)
            dismiss()
        } catch {
            errorMessage = "Could not delete: \(error.localizedDescription)"
        }
    }
}
