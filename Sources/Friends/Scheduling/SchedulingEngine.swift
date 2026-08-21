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

    static func mondayOfWeek(containing date: Date, calendar: Calendar = .current) -> Date {
        var cal = calendar
        cal.firstWeekday = 2 // Monday
        let components = cal.dateComponents([.yearForWeekOfYear, .weekOfYear], from: date)
        return cal.date(from: components)!
    }

    static func computeSchedule(
        lastConnectedDate: Date,
        frequencyDays: Int,
        toggle: DayToggle,
        blockedRanges: [BlockedRange],
        calendar: Calendar = .current
    ) -> ScheduleResult {
        let rawTarget = calendar.date(byAdding: .day, value: frequencyDays, to: lastConnectedDate)!
        var target = snapToToggle(rawTarget, toggle: toggle, calendar: calendar)
        target = avoidBlockedRanges(target, ranges: blockedRanges, toggle: toggle, calendar: calendar)

        let weekday = calendar.component(.weekday, from: target)
        let isWeekend = weekday == 1 || weekday == 7
        let notificationDate = isWeekend
            ? mondayOfWeek(containing: target, calendar: calendar)
            : target

        return ScheduleResult(nextTargetDate: target, notificationDate: notificationDate)
    }

    static func pushMissedSchedule(
        currentTargetDate: Date,
        toggle: DayToggle,
        blockedRanges: [BlockedRange],
        calendar: Calendar = .current
    ) -> ScheduleResult {
        // Walk forward through the current run of toggle-matching days (e.g. Sat+Sun
        // for weekendsOnly) so we don't just land on the very next day if it still
        // matches — that would be "the same missed weekend," not a new opportunity.
        // For .allDays there is no "run" concept (every day matches), so the loop
        // below would never terminate; just advance a single day instead.
        var candidate = currentTargetDate
        if toggle == .allDays {
            candidate = calendar.date(byAdding: .day, value: 1, to: candidate)!
        } else {
            while matches(candidate, toggle: toggle, calendar: calendar) {
                candidate = calendar.date(byAdding: .day, value: 1, to: candidate)!
            }
        }

        var target = snapToToggle(candidate, toggle: toggle, calendar: calendar)
        target = avoidBlockedRanges(target, ranges: blockedRanges, toggle: toggle, calendar: calendar)

        let weekday = calendar.component(.weekday, from: target)
        let isWeekend = weekday == 1 || weekday == 7
        let notificationDate = isWeekend
            ? mondayOfWeek(containing: target, calendar: calendar)
            : target

        return ScheduleResult(nextTargetDate: target, notificationDate: notificationDate)
    }
}
