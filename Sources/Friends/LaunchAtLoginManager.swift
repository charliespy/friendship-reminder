// Sources/Friends/LaunchAtLoginManager.swift
// Wraps SMAppService for enabling/disabling "Launch at Login" — the only
// file that imports ServiceManagement, mirroring how RemindersService is
// the only file that imports EventKit.
import Foundation
import ServiceManagement

enum LaunchAtLoginManager {
    static var isEnabled: Bool {
        SMAppService.mainApp.status == .enabled
    }

    static var requiresApproval: Bool {
        SMAppService.mainApp.status == .requiresApproval
    }

    static var statusDescription: String {
        switch SMAppService.mainApp.status {
        case .enabled: return "Enabled"
        case .notRegistered: return "Not enabled"
        case .requiresApproval: return "Added, but needs approval in System Settings"
        case .notFound: return "Not found"
        @unknown default: return "Unknown"
        }
    }

    /// Registers or unregisters Friends as a login item. Throws if macOS
    /// rejects the request (e.g. the bundle isn't in a state it'll allow).
    static func setEnabled(_ enabled: Bool) throws {
        if enabled {
            if SMAppService.mainApp.status != .enabled {
                try SMAppService.mainApp.register()
            }
        } else if SMAppService.mainApp.status != .notRegistered {
            try SMAppService.mainApp.unregister()
        }
    }

    /// Opens System Settings > General > Login Items, for the case where
    /// registration succeeded but macOS still requires the user's approval.
    static func openLoginItemsSettings() {
        SMAppService.openSystemSettingsLoginItems()
    }
}
