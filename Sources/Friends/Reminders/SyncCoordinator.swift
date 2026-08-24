// Sources/Friends/Reminders/SyncCoordinator.swift
import Foundation

final class SyncCoordinator {
    private let remindersService: RemindersService
    private let calendar: Calendar

    init(remindersService: RemindersService = RemindersService(), calendar: Calendar = .current) {
        self.remindersService = remindersService
        self.calendar = calendar
    }

    private func blockedRanges(from ranges: [BlockedDateRange]) -> [BlockedRange] {
        ranges.map { BlockedRange(start: $0.startDate, end: $0.endDate) }
    }

    func runSyncCycle(people: [Person], blockedRanges ranges: [BlockedDateRange], settings: AppSettings) async throws {
        let granted = try await remindersService.requestAccess()
        guard granted else {
            throw NSError(
                domain: "SyncCoordinator",
                code: 1,
                userInfo: [NSLocalizedDescriptionKey: "Reminders access was not granted. Grant access in System Settings > Privacy & Security > Reminders, then try again."]
            )
        }

        let blocked = blockedRanges(from: ranges)
        let today = calendar.startOfDay(for: Date())

        for person in people {
            // Detect completion first: if the linked reminder is done, advance lastConnectedDate.
            if let identifier = person.reminderIdentifier,
               let completed = try await remindersService.isCompleted(identifier: identifier),
               completed {
                person.lastConnectedDate = today
                let result = SchedulingEngine.computeSchedule(
                    lastConnectedDate: person.lastConnectedDate,
                    frequencyDays: person.frequencyDays,
                    toggle: settings.dayToggle,
                    blockedRanges: blocked,
                    calendar: calendar
                )
                person.nextTargetDate = result.nextTargetDate
                person.notificationDate = result.notificationDate
                person.reminderIdentifier = try remindersService.upsertReminder(
                    identifier: nil,
                    title: "Reach out to \(person.name)",
                    dueDate: person.notificationDate,
                    notes: describeTarget(person.nextTargetDate)
                )
                continue
            }

            // Detect a missed (overdue, not completed) reminder and push it forward.
            // Compare day-granular values so a notificationDate later today isn't
            // mistaken for overdue purely due to time-of-day (see Task 8 review).
            if calendar.startOfDay(for: person.notificationDate) < today {
                let result = SchedulingEngine.pushMissedSchedule(
                    currentTargetDate: person.nextTargetDate,
                    toggle: settings.dayToggle,
                    blockedRanges: blocked,
                    calendar: calendar
                )
                person.nextTargetDate = result.nextTargetDate
                person.notificationDate = result.notificationDate
            }

            // Ensure a reminder exists and is up to date. upsertReminder recreates it
            // if the stored identifier no longer resolves (the "deleted but not
            // completed" edge case from the spec), and updates it in place otherwise —
            // including cases where a newly added blocked range shifted the dates above.
            person.reminderIdentifier = try remindersService.upsertReminder(
                identifier: person.reminderIdentifier,
                title: "Reach out to \(person.name)",
                dueDate: person.notificationDate,
                notes: describeTarget(person.nextTargetDate)
            )
        }
    }

    private func describeTarget(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        return "Aim for \(formatter.string(from: date))"
    }

    /// Removes a person's linked reminder (if any) ahead of deleting the
    /// `Person` itself from the model. Safe to call even if the reminder was
    /// already removed or never created.
    func deletePerson(_ person: Person) async throws {
        let granted = try await remindersService.requestAccess()
        guard granted else {
            throw NSError(
                domain: "SyncCoordinator",
                code: 1,
                userInfo: [NSLocalizedDescriptionKey: "Reminders access was not granted. Grant access in System Settings > Privacy & Security > Reminders, then try again."]
            )
        }
        if let identifier = person.reminderIdentifier {
            try remindersService.deleteReminder(identifier: identifier)
        }
    }

    /// Wipes every reminder in the Friends list and rebuilds one fresh
    /// reminder per person from their current data — a full reset for when
    /// reminders have drifted (manually deleted, edited, duplicated, etc.)
    /// rather than an incremental sync. Recomputes each person's schedule
    /// from their current `lastConnectedDate` rather than reusing whatever
    /// `nextTargetDate`/`notificationDate` was last stored, so the result
    /// reflects "what's relevant today" under the current toggle/blocked
    /// ranges, not stale state that may have been superseded.
    func resyncAll(people: [Person], blockedRanges ranges: [BlockedDateRange], settings: AppSettings) async throws {
        let granted = try await remindersService.requestAccess()
        guard granted else {
            throw NSError(
                domain: "SyncCoordinator",
                code: 1,
                userInfo: [NSLocalizedDescriptionKey: "Reminders access was not granted. Grant access in System Settings > Privacy & Security > Reminders, then try again."]
            )
        }

        try await remindersService.deleteAllReminders()

        let blocked = blockedRanges(from: ranges)
        for person in people {
            let result = SchedulingEngine.computeSchedule(
                lastConnectedDate: person.lastConnectedDate,
                frequencyDays: person.frequencyDays,
                toggle: settings.dayToggle,
                blockedRanges: blocked,
                calendar: calendar
            )
            person.nextTargetDate = result.nextTargetDate
            person.notificationDate = result.notificationDate
            person.reminderIdentifier = try remindersService.upsertReminder(
                identifier: nil,
                title: "Reach out to \(person.name)",
                dueDate: person.notificationDate,
                notes: describeTarget(person.nextTargetDate)
            )
        }
    }
}
