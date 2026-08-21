// Sources/Friends/Models/AppSettings.swift
import Foundation
import Combine

final class AppSettings: ObservableObject {
    static let shared = AppSettings()

    private let defaultsKey = "dayToggle"
    private let defaults: UserDefaults

    @Published var dayToggle: DayToggle {
        didSet {
            defaults.set(dayToggle.rawValue, forKey: defaultsKey)
        }
    }

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        let stored = defaults.string(forKey: defaultsKey)
        self.dayToggle = stored.flatMap(DayToggle.init(rawValue:)) ?? .weekendsOnly
    }
}
