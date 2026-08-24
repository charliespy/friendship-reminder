// Sources/Friends/Views/PeopleListView.swift
// The "People" preferences tab — browse everyone in Friends, see their
// status at a glance, and tap into EditPersonView to edit or delete them.
import SwiftUI
import SwiftData

struct PeopleListView: View {
    @Query private var people: [Person]
    @State private var personToEdit: Person?

    private var sorted: [Person] {
        people.sorted { lhs, rhs in
            if lhs.notificationDate != rhs.notificationDate {
                return lhs.notificationDate < rhs.notificationDate
            }
            return lhs.name < rhs.name
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            VStack(alignment: .leading, spacing: 2) {
                Text("People").font(FriendsTheme.sectionTitle).foregroundStyle(FriendsTheme.ink)
                Text("Tap anyone to edit their details or remove them.")
                    .font(FriendsTheme.body(12))
                    .foregroundStyle(FriendsTheme.inkSoft)
            }

            if sorted.isEmpty {
                Text("Nobody yet — add someone from the Add Person tab.")
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
    }

    private func row(for person: Person) -> some View {
        let isOverdue = person.notificationDate < Date()
        return Button {
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
            .friendsCard(padding: 9)
        }
        .buttonStyle(.plain)
    }

    private func formatted(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        return formatter.string(from: date)
    }
}
