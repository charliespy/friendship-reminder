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
}
