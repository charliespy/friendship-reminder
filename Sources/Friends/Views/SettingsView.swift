// Sources/Friends/Views/SettingsView.swift
import SwiftUI

struct SettingsView: View {
    @ObservedObject private var settings = AppSettings.shared

    var body: some View {
        Form {
            Picker("Meeting days", selection: $settings.dayToggle) {
                ForEach(DayToggle.allCases) { toggle in
                    Text(toggle.displayName).tag(toggle)
                }
            }
            .pickerStyle(.radioGroup)
        }
        .padding()
    }
}
