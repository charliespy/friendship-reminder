// Sources/Friends/Views/MenuBarDropdownView.swift
import SwiftUI
import SwiftData

struct MenuBarDropdownView: View {
    @Query private var people: [Person]
    @Environment(\.openSettings) private var openSettings

    private var dueSoon: [Person] {
        let today = Date()
        return people
            .filter { $0.notificationDate <= Calendar.current.date(byAdding: .day, value: 7, to: today)! }
            .sorted { lhs, rhs in
                if lhs.notificationDate != rhs.notificationDate {
                    return lhs.notificationDate < rhs.notificationDate
                }
                return lhs.frequencyDays < rhs.frequencyDays
            }
    }

    private var allPeopleSorted: [Person] {
        people.sorted { $0.name < $1.name }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Friends").font(.headline)

            Divider()

            Text("Due Soon").font(.subheadline).foregroundStyle(.secondary)
            if dueSoon.isEmpty {
                Text("Nothing due in the next week.").font(.caption)
            } else {
                ForEach(dueSoon) { person in
                    dueSoonRow(person)
                }
            }

            Divider()

            Text("All People (\(people.count))").font(.subheadline).foregroundStyle(.secondary)
            ForEach(allPeopleSorted) { person in
                Text(person.name).font(.caption)
            }

            Divider()

            HStack {
                Button("Preferences…") { openSettings() }
            }
        }
        .padding()
        .frame(minWidth: 280)
    }

    @ViewBuilder
    private func dueSoonRow(_ person: Person) -> some View {
        let isOverdue = person.notificationDate < Date()
        HStack {
            Text(person.name).bold()
            Spacer()
            if isOverdue {
                Text("Overdue since \(formatted(person.notificationDate))")
                    .font(.caption)
                    .foregroundStyle(.red)
            } else {
                Text("Notify \(formatted(person.notificationDate))")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private func formatted(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        return formatter.string(from: date)
    }
}
