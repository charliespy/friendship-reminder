// Sources/Friends/Models/DayToggle.swift
import Foundation

enum DayToggle: String, Codable, CaseIterable, Identifiable {
    case allDays
    case weekdaysOnly
    case weekendsOnly

    var id: String { rawValue }

    var displayName: String {
        switch self {
        case .allDays: return "All days"
        case .weekdaysOnly: return "Weekdays only"
        case .weekendsOnly: return "Weekends only"
        }
    }
}
