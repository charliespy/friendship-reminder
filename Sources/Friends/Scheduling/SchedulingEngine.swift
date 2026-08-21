// Sources/Friends/Scheduling/SchedulingEngine.swift
import Foundation

struct BlockedRange: Equatable {
    let start: Date
    let end: Date
}

struct ScheduleResult: Equatable {
    let nextTargetDate: Date
    let notificationDate: Date
}

enum SchedulingEngine {
    static func matches(_ date: Date, toggle: DayToggle, calendar: Calendar = .current) -> Bool {
        let weekday = calendar.component(.weekday, from: date) // Sunday = 1 ... Saturday = 7
        let isWeekend = weekday == 1 || weekday == 7
        switch toggle {
        case .allDays: return true
        case .weekendsOnly: return isWeekend
        case .weekdaysOnly: return !isWeekend
        }
    }

    static func snapToToggle(_ date: Date, toggle: DayToggle, calendar: Calendar = .current) -> Date {
        var candidate = date
        while !matches(candidate, toggle: toggle, calendar: calendar) {
            candidate = calendar.date(byAdding: .day, value: 1, to: candidate)!
        }
        return candidate
    }

    static func avoidBlockedRanges(
        _ date: Date,
        ranges: [BlockedRange],
        toggle: DayToggle,
        calendar: Calendar = .current
    ) -> Date {
        var candidate = date
        var changed = true
        while changed {
            changed = false
            for range in ranges {
                if candidate >= range.start && candidate <= range.end {
                    let dayAfterRange = calendar.date(byAdding: .day, value: 1, to: range.end)!
                    candidate = snapToToggle(dayAfterRange, toggle: toggle, calendar: calendar)
                    changed = true
                }
            }
        }
        return candidate
    }
}
