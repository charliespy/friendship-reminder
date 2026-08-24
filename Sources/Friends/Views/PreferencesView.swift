// Sources/Friends/Views/PreferencesView.swift
import SwiftUI

private enum PreferencesTab: String, CaseIterable, Identifiable {
    case people, blockedDates, settings

    var id: String { rawValue }

    var title: String {
        switch self {
        case .people: return "People"
        case .blockedDates: return "Blocked Dates"
        case .settings: return "Settings"
        }
    }

    var systemImage: String {
        switch self {
        case .people: return "person.2"
        case .blockedDates: return "calendar.badge.exclamationmark"
        case .settings: return "gearshape"
        }
    }
}

struct PreferencesView: View {
    @State private var selectedTab: PreferencesTab = .people

    var body: some View {
        HStack(spacing: 0) {
            sidebar

            Divider().overlay(FriendsTheme.hairline)

            ScrollView {
                content.padding(20)
            }
            .frame(maxWidth: .infinity, alignment: .topLeading)
            .background(FriendsTheme.cream)
        }
        .frame(minWidth: 560, idealWidth: 640, maxWidth: .infinity, minHeight: 420, idealHeight: 480, maxHeight: .infinity)
    }

    private var sidebar: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(spacing: 8) {
                ZStack {
                    Circle().fill(FriendsTheme.rust.gradient).frame(width: 22, height: 22)
                    Image(systemName: "person.2.fill").font(.system(size: 10, weight: .semibold)).foregroundStyle(.white)
                }
                Text("Friends").font(FriendsTheme.sectionTitle).foregroundStyle(FriendsTheme.ink)
            }
            .padding(.horizontal, 8)
            .padding(.bottom, 14)
            .padding(.top, 4)

            ForEach(PreferencesTab.allCases) { tab in
                SidebarButton(
                    title: tab.title,
                    systemImage: tab.systemImage,
                    isSelected: tab == selectedTab
                ) {
                    selectedTab = tab
                }
            }

            Spacer()
        }
        .padding(12)
        .frame(width: 168)
        .background(FriendsTheme.paper)
    }

    @ViewBuilder
    private var content: some View {
        switch selectedTab {
        case .people: PeopleListView()
        case .blockedDates: BlockedDatesView()
        case .settings: SettingsView()
        }
    }
}
