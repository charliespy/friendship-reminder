// Sources/Friends/Views/PeopleListView.swift
// The "People" preferences tab — the hub for the whole roster: add people,
// tap a row to edit/delete one, or check multiple rows to bulk-delete them.
import SwiftUI
import SwiftData

struct PeopleListView: View {
    @Environment(\.modelContext) private var modelContext
    @ObservedObject private var settings = AppSettings.shared
    @Query private var people: [Person]
    @Query private var blockedRanges: [BlockedDateRange]

    @State private var personToEdit: Person?
    @State private var showAddPerson = false
    @State private var selectedIDs: Set<Person.ID> = []
    @State private var showBulkDeleteConfirmation = false
    @State private var isBulkDeleting = false
    @State private var errorMessage: String?

    private var sorted: [Person] {
        people.sorted { lhs, rhs in
            if lhs.notificationDate != rhs.notificationDate {
                return lhs.notificationDate < rhs.notificationDate
            }
            return lhs.name < rhs.name
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            header

            if !selectedIDs.isEmpty {
                selectionBar
            }

            if let errorMessage {
                HStack(alignment: .top, spacing: 6) {
                    Image(systemName: "exclamationmark.triangle.fill").font(.system(size: 11))
                    Text(errorMessage).font(FriendsTheme.body(12))
                }
                .foregroundStyle(FriendsTheme.rust)
            }

            if sorted.isEmpty {
                Text("Nobody yet — add someone to get started.")
                    .font(FriendsTheme.body(12))
                    .foregroundStyle(FriendsTheme.inkFaint)
            } else {
                VStack(spacing: 6) {
                    ForEach(sorted) { person in
                        row(for: person)
                    }
                }
            }

            Spacer()
        }
        .sheet(item: $personToEdit) { person in
            EditPersonView(person: person)
        }
        .sheet(isPresented: $showAddPerson) {
            AddPersonView()
        }
        .task { await syncOnAppear() }
    }

    /// Opening the People tab is often the moment a stale/overdue reminder
    /// actually gets noticed — sync immediately rather than waiting for the
    /// next scheduled launch/wake/hourly trigger, so a person who's been
    /// overdue for a long stretch gets caught up (and re-titled) right away.
    private func syncOnAppear() async {
        do {
            let coordinator = SyncCoordinator()
            try await coordinator.runSyncCycle(people: people, blockedRanges: blockedRanges, settings: settings)
        } catch {
            errorMessage = "Could not sync: \(error.localizedDescription)"
        }
    }

    private var header: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 2) {
                Text("People").font(FriendsTheme.sectionTitle).foregroundStyle(FriendsTheme.ink)
                Text("Check the box to select people, or tap a row to edit.")
                    .font(FriendsTheme.body(12))
                    .foregroundStyle(FriendsTheme.inkSoft)
            }
            Spacer()
            Button {
                showAddPerson = true
            } label: {
                HStack(spacing: 5) {
                    Image(systemName: "plus")
                    Text("Add Person")
                }
            }
            .buttonStyle(PrimaryButtonStyle())
        }
    }

    private var selectionBar: some View {
        HStack {
            Text("\(selectedIDs.count) selected")
                .font(FriendsTheme.body(12, weight: .semibold))
                .foregroundStyle(FriendsTheme.inkSoft)
            Spacer()
            Button("Deselect All") { selectedIDs.removeAll() }
                .buttonStyle(QuietButtonStyle())
                .disabled(isBulkDeleting)
            Button {
                showBulkDeleteConfirmation = true
            } label: {
                HStack(spacing: 5) {
                    if isBulkDeleting {
                        ProgressView().controlSize(.small)
                    } else {
                        Image(systemName: "trash")
                    }
                    Text("Delete Selected")
                }
            }
            .buttonStyle(PrimaryButtonStyle(tint: FriendsTheme.rust, isEnabled: !isBulkDeleting))
            .disabled(isBulkDeleting)
        }
        .confirmationDialog(
            "Delete \(selectedIDs.count) \(selectedIDs.count == 1 ? "person" : "people")?",
            isPresented: $showBulkDeleteConfirmation,
            titleVisibility: .visible
        ) {
            Button("Delete", role: .destructive) {
                Task { await bulkDelete() }
            }
        } message: {
            Text("This also removes their reminders from the Friends list in Reminders.")
        }
    }

    private func row(for person: Person) -> some View {
        let isOverdue = person.notificationDate < Date()
        let isSelected = selectedIDs.contains(person.id)
        return HStack(spacing: 10) {
            Button {
                toggleSelection(person)
            } label: {
                Image(systemName: isSelected ? "checkmark.circle.fill" : "circle")
                    .font(.system(size: 16))
                    .foregroundStyle(isSelected ? FriendsTheme.rust : FriendsTheme.inkFaint)
            }
            .buttonStyle(.plain)

            Button {
                personToEdit = person
            } label: {
                HStack(spacing: 10) {
                    AvatarBadge(name: person.name, diameter: 28)
                    VStack(alignment: .leading, spacing: 1) {
                        Text(person.name)
                            .font(FriendsTheme.body(13, weight: .semibold))
                            .foregroundStyle(FriendsTheme.ink)
                        Text("\(FrequencyOption.label(forDays: person.frequencyDays)) · \(isOverdue ? "Overdue since" : "Next") \(formatted(person.notificationDate))")
                            .font(FriendsTheme.body(11))
                            .foregroundStyle(isOverdue ? FriendsTheme.rust : FriendsTheme.inkSoft)
                    }
                    Spacer()
                    Image(systemName: "chevron.right")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(FriendsTheme.inkFaint)
                }
            }
            .buttonStyle(.plain)
        }
        .friendsCard(padding: 9)
    }

    private func toggleSelection(_ person: Person) {
        if selectedIDs.contains(person.id) {
            selectedIDs.remove(person.id)
        } else {
            selectedIDs.insert(person.id)
        }
    }

    private func bulkDelete() async {
        isBulkDeleting = true
        defer { isBulkDeleting = false }

        let toDelete = sorted.filter { selectedIDs.contains($0.id) }
        let coordinator = SyncCoordinator()
        var failures: [String] = []

        for person in toDelete {
            do {
                try await coordinator.deletePerson(person)
                modelContext.delete(person)
            } catch {
                failures.append(person.name)
            }
        }

        selectedIDs.removeAll()
        errorMessage = failures.isEmpty
            ? nil
            : "Could not remove reminders for: \(failures.joined(separator: ", ")). They were left in Friends so you can retry."
    }

    private func formatted(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        return formatter.string(from: date)
    }
}
