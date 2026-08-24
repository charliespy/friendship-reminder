// Sources/Friends/Views/MenuBarDropdownView.swift
import SwiftUI
import SwiftData
import AppKit

struct MenuBarDropdownView: View {
    @Query private var people: [Person]
    @Environment(\.openSettings) private var openSettings

    private var dueSoon: [Person] {
        let today = Date()
        return people
            .filter { $0.notificationDate <= (Calendar.current.date(byAdding: .day, value: 7, to: today) ?? today) }
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
        VStack(alignment: .leading, spacing: 0) {
            header

            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    dueSoonSection
                    allPeopleSection
                }
                .padding(.horizontal, 16)
                .padding(.top, 4)
            }
            .frame(maxHeight: 320)

            footer
        }
        .padding(.bottom, 4)
        .frame(minWidth: 300)
        .background(FriendsTheme.cream)
    }

    // MARK: Header

    private var header: some View {
        HStack(spacing: 10) {
            ZStack {
                Circle().fill(FriendsTheme.rust.gradient).frame(width: 30, height: 30)
                Image(systemName: "person.2.fill")
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(.white)
            }
            VStack(alignment: .leading, spacing: 1) {
                Text("Friends").font(FriendsTheme.wordmark).foregroundStyle(FriendsTheme.ink)
                Text(subtitle).font(FriendsTheme.body(11)).foregroundStyle(FriendsTheme.inkSoft)
            }
            Spacer()
        }
        .padding(.horizontal, 16)
        .padding(.top, 14)
        .padding(.bottom, 10)
    }

    private var subtitle: String {
        let count = people.count
        let dueCount = dueSoon.count
        let peopleWord = count == 1 ? "person" : "people"
        if dueCount == 0 {
            return "\(count) \(peopleWord)"
        }
        return "\(count) \(peopleWord) · \(dueCount) due soon"
    }

    // MARK: Due Soon

    private var dueSoonSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionLabel(text: "Due Soon", color: FriendsTheme.rust)
            if dueSoon.isEmpty {
                Text("Nothing due in the next week.")
                    .font(FriendsTheme.body(12))
                    .foregroundStyle(FriendsTheme.inkFaint)
                    .padding(.vertical, 2)
            } else {
                VStack(spacing: 6) {
                    ForEach(dueSoon) { person in
                        dueSoonRow(person)
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func dueSoonRow(_ person: Person) -> some View {
        let isOverdue = person.notificationDate < Date()
        HStack(spacing: 10) {
            AvatarBadge(name: person.name, diameter: 26)
            VStack(alignment: .leading, spacing: 1) {
                Text(person.name)
                    .font(FriendsTheme.body(13, weight: .semibold))
                    .foregroundStyle(FriendsTheme.ink)
                Text(isOverdue ? "Overdue since \(formatted(person.notificationDate))" : "Notify \(formatted(person.notificationDate))")
                    .font(FriendsTheme.body(11))
                    .foregroundStyle(isOverdue ? FriendsTheme.rust : FriendsTheme.inkSoft)
            }
            Spacer()
            Circle()
                .fill(isOverdue ? FriendsTheme.rust : FriendsTheme.amber)
                .frame(width: 6, height: 6)
        }
        .friendsCard(padding: 9)
    }

    // MARK: All People

    private var allPeopleSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            SectionLabel(text: "All People (\(people.count))")
            if allPeopleSorted.isEmpty {
                Text("Add someone to get started.")
                    .font(FriendsTheme.body(12))
                    .foregroundStyle(FriendsTheme.inkFaint)
                    .padding(.vertical, 2)
            } else {
                VStack(spacing: 0) {
                    ForEach(Array(allPeopleSorted.enumerated()), id: \.element.id) { index, person in
                        HStack(spacing: 10) {
                            AvatarBadge(name: person.name, diameter: 20)
                            Text(person.name)
                                .font(FriendsTheme.body(12))
                                .foregroundStyle(FriendsTheme.ink)
                            Spacer()
                        }
                        .padding(.vertical, 6)
                        if index < allPeopleSorted.count - 1 {
                            Divider().overlay(FriendsTheme.hairline)
                        }
                    }
                }
            }
        }
    }

    // MARK: Footer

    private var footer: some View {
        VStack(spacing: 0) {
            Divider().overlay(FriendsTheme.hairline)
            HStack {
                Spacer()
                Button {
                    openSettings()
                    NSApp.activate(ignoringOtherApps: true)
                } label: {
                    HStack(spacing: 5) {
                        Image(systemName: "gearshape")
                        Text("Preferences…")
                    }
                }
                .buttonStyle(QuietButtonStyle())
            }
            .padding(.horizontal, 12)
            .padding(.top, 8)
        }
    }

    private func formatted(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        return formatter.string(from: date)
    }
}
