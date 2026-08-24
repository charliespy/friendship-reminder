// Sources/Friends/Views/SettingsView.swift
import SwiftUI

struct SettingsView: View {
    @ObservedObject private var settings = AppSettings.shared

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
}
