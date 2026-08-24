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
        try await requireAccess()

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
                    title: reminderTitle(for: person, isOverdue: false),
                    dueDate: person.notificationDate,
                    notes: describeTarget(person.nextTargetDate)
                )
                continue
            }

            // Detect a missed (overdue, not completed) reminder and push it forward.
            // Compare day-granular values so a notificationDate later today isn't
            // mistaken for overdue purely due to time-of-day (see Task 8 review).
            let wasOverdue = calendar.startOfDay(for: person.notificationDate) < today
            if wasOverdue {
                var result = SchedulingEngine.pushMissedSchedule(
                    currentTargetDate: person.nextTargetDate,
                    toggle: settings.dayToggle,
                    blockedRanges: blocked,
                    calendar: calendar
                )
                // A single push only advances by one cycle. If the app went
                // unsynced for a long stretch (weeks/months), that would leave
                // the person still overdue — visibly stuck as overdue in
                // Reminders — until enough individual syncs happened to walk
                // them forward one cycle at a time. Keep pushing until the
                // notification date is genuinely caught up to today, landing
                // on the next real available day instead of a still-past one.
                // Each iteration strictly advances the date, so this always
                // terminates for any realistic backlog; the cap only guards
                // against a future scheduling bug reintroducing non-termination.
                var iterations = 0
                while calendar.startOfDay(for: result.notificationDate) < today, iterations < 1000 {
                    result = SchedulingEngine.pushMissedSchedule(
                        currentTargetDate: result.nextTargetDate,
                        toggle: settings.dayToggle,
                        blockedRanges: blocked,
                        calendar: calendar
                    )
                    iterations += 1
                }
                person.nextTargetDate = result.nextTargetDate
                person.notificationDate = result.notificationDate
            }

            // Ensure a reminder exists and is up to date. upsertReminder recreates it
            // if the stored identifier no longer resolves (the "deleted but not
            // completed" edge case from the spec), and updates it in place otherwise —
            // including cases where a newly added blocked range shifted the dates above.
            person.reminderIdentifier = try remindersService.upsertReminder(
                identifier: person.reminderIdentifier,
                title: reminderTitle(for: person, isOverdue: wasOverdue),
                dueDate: person.notificationDate,
                notes: describeTarget(person.nextTargetDate)
            )
        }
    }

    private func reminderTitle(for person: Person, isOverdue: Bool) -> String {
        isOverdue ? "Reach out to \(person.name) (Overdue)" : "Reach out to \(person.name)"
    }

    private func describeTarget(_ date: Date) -> String {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        return "Aim for \(formatter.string(from: date))"
    }

    private func requireAccess() async throws {
        let granted = try await remindersService.requestAccess()
        guard granted else {
            throw NSError(
                domain: "SyncCoordinator",
                code: 1,
                userInfo: [NSLocalizedDescriptionKey: "Reminders access was not granted. Grant access in System Settings > Privacy & Security > Reminders, then try again."]
            )
        }
    }

    /// Removes a person's linked reminder (if any) ahead of deleting the
    /// `Person` itself from the model. Safe to call even if the reminder was
    /// already removed or never created.
    func deletePerson(_ person: Person) async throws {
        try await requireAccess()
        if let identifier = person.reminderIdentifier {
            try remindersService.deleteReminder(identifier: identifier)
        }
    }

    /// Removes every reminder currently in the Friends list, and clears each
    /// person's now-stale `reminderIdentifier` so a later `createAllReminders`
    /// or `runSyncCycle` creates fresh ones instead of trying to update
    /// reminders that no longer exist.
    func deleteAllReminders(people: [Person]) async throws {
        try await requireAccess()
        try await remindersService.deleteAllReminders()
        for person in people {
            person.reminderIdentifier = nil
        }
    }

    /// Recomputes every person's schedule from their current
    /// `lastConnectedDate` (so the result reflects what's relevant *today*
    /// under the current toggle/blocked ranges, not stale prior state) and
    /// ensures each has a reminder. Uses each person's existing
    /// `reminderIdentifier` — updating it in place if still valid, or
    /// creating a fresh one if it's nil/stale (e.g. after `deleteAllReminders`)
    /// — so running this on its own never creates duplicates.
    func createAllReminders(people: [Person], blockedRanges ranges: [BlockedDateRange], settings: AppSettings) async throws {
        try await requireAccess()

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
                identifier: person.reminderIdentifier,
                title: "Reach out to \(person.name)",
                dueDate: person.notificationDate,
                notes: describeTarget(person.nextTargetDate)
            )
        }
    }
}
