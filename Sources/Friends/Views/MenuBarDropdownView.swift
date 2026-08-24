// Sources/Friends/Views/MenuBarDropdownView.swift
import SwiftUI
import AppKit

struct MenuBarDropdownView: View {
    @Environment(\.openSettings) private var openSettings

    var body: some View {
        Button {
            openSettings()
            NSApp.activate(ignoringOtherApps: true)
        } label: {
            HStack(spacing: 6) {
                Image(systemName: "gearshape")
                Text("Preferences…")
                Spacer()
            }
        }
        .buttonStyle(QuietButtonStyle())
        .padding(8)
        .frame(width: 180)
        .background(FriendsTheme.cream)
    }
}
