// Sources/Friends/Views/FrequencyOption.swift
// Shared presentation metadata mapping a Person's stored `frequencyDays`
// to a human-readable label. Used by both AddPersonView and EditPersonView
// so the two forms can never drift out of sync with each other.
import Foundation

enum FrequencyOption {
    static let all: [(label: String, days: Int)] = [
        ("Monthly", 30), ("Quarterly", 90), ("Semiannual", 180), ("Yearly", 365)
    ]

    static func label(forDays days: Int) -> String {
        all.first(where: { $0.days == days })?.label ?? "Every \(days) days"
    }
}
