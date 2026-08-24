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

            // Detect a missed (overdue, not completed) reminder and push it
            // forward until it's genuinely caught up to today.
            let caughtUp = catchUpIfOverdue(
                nextTargetDate: person.nextTargetDate,
                notificationDate: person.notificationDate,
                toggle: settings.dayToggle,
                blocked: blocked,
                today: today
            )
            person.nextTargetDate = caughtUp.nextTargetDate
            person.notificationDate = caughtUp.notificationDate

            // Ensure a reminder exists and is up to date. upsertReminder recreates it
            // if the stored identifier no longer resolves (the "deleted but not
            // completed" edge case from the spec), and updates it in place otherwise —
            // including cases where a newly added blocked range shifted the dates above.
            person.reminderIdentifier = try remindersService.upsertReminder(
                identifier: person.reminderIdentifier,
                title: reminderTitle(for: person, isOverdue: caughtUp.wasOverdue),
                dueDate: person.notificationDate,
                notes: describeTarget(person.nextTargetDate)
            )
        }
    }

    private func reminderTitle(for person: Person, isOverdue: Bool) -> String {
        isOverdue ? "Reach out to \(person.name) (Overdue)" : "Reach out to \(person.name)"
    }

    /// If `notificationDate` is already in the past relative to `today`,
    /// repeatedly pushes it forward (`SchedulingEngine.pushMissedSchedule`)
    /// until it's genuinely caught up, rather than leaving it "stuck"
    /// overdue after only a single push — or, for a freshly computed
    /// schedule (e.g. from `createAllReminders`), catching a `lastConnectedDate`
    /// far enough in the past that even a fresh `computeSchedule` result
    /// still lands before today. Each iteration strictly advances the date,
    /// so this always terminates for any realistic backlog; the cap only
    /// guards against a future scheduling bug reintroducing non-termination.
    private func catchUpIfOverdue(
        nextTargetDate: Date,
        notificationDate: Date,
        toggle: DayToggle,
        blocked: [BlockedRange],
        today: Date
    ) -> (nextTargetDate: Date, notificationDate: Date, wasOverdue: Bool) {
        guard calendar.startOfDay(for: notificationDate) < today else {
            return (nextTargetDate, notificationDate, false)
        }

        var result = ScheduleResult(nextTargetDate: nextTargetDate, notificationDate: notificationDate)
        var iterations = 0
        while calendar.startOfDay(for: result.notificationDate) < today, iterations < 1000 {
            result = SchedulingEngine.pushMissedSchedule(
                currentTargetDate: result.nextTargetDate,
                toggle: toggle,
                blockedRanges: blocked,
                calendar: calendar
            )
            iterations += 1
        }
        return (result.nextTargetDate, result.notificationDate, true)
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
        let today = calendar.startOfDay(for: Date())
        for person in people {
            let result = SchedulingEngine.computeSchedule(
                lastConnectedDate: person.lastConnectedDate,
                frequencyDays: person.frequencyDays,
                toggle: settings.dayToggle,
                blockedRanges: blocked,
                calendar: calendar
            )
            let caughtUp = catchUpIfOverdue(
                nextTargetDate: result.nextTargetDate,
                notificationDate: result.notificationDate,
                toggle: settings.dayToggle,
                blocked: blocked,
                today: today
            )
            person.nextTargetDate = caughtUp.nextTargetDate
            person.notificationDate = caughtUp.notificationDate
            person.reminderIdentifier = try remindersService.upsertReminder(
                identifier: person.reminderIdentifier,
                title: reminderTitle(for: person, isOverdue: caughtUp.wasOverdue),
                dueDate: person.notificationDate,
                notes: describeTarget(person.nextTargetDate)
            )
        }
    }
}
