// Sources/Friends/Views/MenuBarDropdownView.swift
import SwiftUI
import AppKit

struct MenuBarDropdownView: View {
    @Environment(\.openSettings) private var openSettings

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 10) {
                ZStack {
                    Circle().fill(FriendsTheme.rust.gradient).frame(width: 26, height: 26)
                    Image(systemName: "person.2.fill")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(.white)
                }
                Text("Friends").font(FriendsTheme.wordmark).foregroundStyle(FriendsTheme.ink)
                Spacer()
            }
            .padding(.horizontal, 14)
            .padding(.top, 12)
            .padding(.bottom, 10)

            Divider().overlay(FriendsTheme.hairline)

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
            .padding(10)
        }
        .frame(width: 220)
        .background(FriendsTheme.cream)
    }
}
