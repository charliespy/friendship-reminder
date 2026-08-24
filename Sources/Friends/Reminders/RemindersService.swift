import EventKit
import Foundation

final class RemindersService {
    let eventStore: EKEventStore
    let listName: String

    init(eventStore: EKEventStore = EKEventStore(), listName: String = "Friends") {
        self.eventStore = eventStore
        self.listName = listName
    }

    func requestAccess() async throws -> Bool {
        try await eventStore.requestFullAccessToReminders()
    }

    func ensureFriendsListExists() throws -> EKCalendar {
        let existing = eventStore.calendars(for: .reminder).first { $0.title == listName }
        if let existing {
            return existing
        }
        let calendar = EKCalendar(for: .reminder, eventStore: eventStore)
        calendar.title = listName
        guard let source = eventStore.defaultCalendarForNewReminders()?.source
            ?? eventStore.sources.first(where: { $0.sourceType == .local })
            ?? eventStore.sources.first
        else {
            throw NSError(domain: "RemindersService", code: 1, userInfo: [NSLocalizedDescriptionKey: "No EKSource available for new calendar"])
        }
        calendar.source = source
        try eventStore.saveCalendar(calendar, commit: true)
        return calendar
    }

    func upsertReminder(identifier: String?, title: String, dueDate: Date, notes: String?) throws -> String {
        let calendar = try ensureFriendsListExists()
        let reminder: EKReminder
        if let identifier, let existing = eventStore.calendarItem(withIdentifier: identifier) as? EKReminder {
            reminder = existing
        } else {
            reminder = EKReminder(eventStore: eventStore)
            reminder.calendar = calendar
        }
        reminder.title = title
        reminder.notes = notes
        let components = Calendar.current.dateComponents([.year, .month, .day], from: dueDate)
        reminder.dueDateComponents = components
        try eventStore.save(reminder, commit: true)
        return reminder.calendarItemIdentifier
    }

    func fetchFriendsListReminders() async throws -> [EKReminder] {
        let calendar = try ensureFriendsListExists()
        let predicate = eventStore.predicateForReminders(in: [calendar])
        return await withCheckedContinuation { continuation in
            eventStore.fetchReminders(matching: predicate) { reminders in
                continuation.resume(returning: reminders ?? [])
            }
        }
    }

    func isCompleted(identifier: String) async throws -> Bool? {
        guard let reminder = eventStore.calendarItem(withIdentifier: identifier) as? EKReminder else {
            return nil
        }
        return reminder.isCompleted
    }

    /// Removes a single reminder by identifier. A no-op (not an error) if the
    /// identifier no longer resolves to a reminder — it's already gone.
    func deleteReminder(identifier: String) throws {
        guard let reminder = eventStore.calendarItem(withIdentifier: identifier) as? EKReminder else {
            return
        }
        try eventStore.remove(reminder, commit: true)
    }

    /// Removes every reminder currently in the Friends list, regardless of
    /// which Person (if any) still references it. Used by the "Resync All"
    /// maintenance action to guarantee a clean rebuild with no leftovers.
    func deleteAllReminders() async throws {
        let reminders = try await fetchFriendsListReminders()
        for reminder in reminders {
            try eventStore.remove(reminder, commit: false)
        }
        try eventStore.commit()
    }
}
