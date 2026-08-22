// Sources/Friends/Views/PreferencesView.swift
import SwiftUI

struct PreferencesView: View {
    var body: some View {
        TabView {
            AddPersonView()
                .tabItem { Label("Add Person", systemImage: "person.badge.plus") }
            BlockedDatesView()
                .tabItem { Label("Blocked Dates", systemImage: "calendar.badge.exclamationmark") }
            SettingsView()
                .tabItem { Label("Settings", systemImage: "gearshape") }
        }
        .frame(width: 420, height: 360)
    }
}
