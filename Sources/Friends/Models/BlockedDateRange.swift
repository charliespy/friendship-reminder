// Sources/Friends/Models/BlockedDateRange.swift
import Foundation
import SwiftData

@Model
final class BlockedDateRange {
    var startDate: Date
    var endDate: Date
    var label: String

    init(startDate: Date, endDate: Date, label: String) {
        self.startDate = startDate
        self.endDate = endDate
        self.label = label
    }
}
