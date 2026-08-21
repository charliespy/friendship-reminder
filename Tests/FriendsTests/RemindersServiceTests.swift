import XCTest
import EventKit
@testable import Friends

final class RemindersServiceTests: XCTestCase {
    var service: RemindersService!
    var store: EKEventStore!

    override func setUp() async throws {
        try await super.setUp()
        store = EKEventStore()
        service = RemindersService(eventStore: store, listName: "Friends-Test-\(UUID().uuidString)")
        let granted = try await service.requestAccess()
        try XCTSkipUnless(granted, "Reminders access not granted in this environment")
    }

    override func tearDown() async throws {
        // Clean up the test list's reminders and the list itself
        if let calendar = try? service.ensureFriendsListExists() {
            let predicate = store.predicateForReminders(in: [calendar])
            let reminders = await withCheckedContinuation { continuation in
                store.fetchReminders(matching: predicate) { continuation.resume(returning: $0 ?? []) }
            }
            for reminder in reminders {
                try? store.remove(reminder, commit: false)
            }
            try? store.commit()
            try? store.removeCalendar(calendar, commit: true)
        }
        try await super.tearDown()
    }

    func test_ensureFriendsListExists_createsListIfMissing() throws {
        let calendar = try service.ensureFriendsListExists()
        XCTAssertEqual(calendar.title, service.listName)
    }

    func test_ensureFriendsListExists_reusesExistingList() throws {
        let first = try service.ensureFriendsListExists()
        let second = try service.ensureFriendsListExists()
        XCTAssertEqual(first.calendarIdentifier, second.calendarIdentifier)
    }

    func test_upsertReminder_createsNewReminderWhenIdentifierIsNil() async throws {
        let identifier = try service.upsertReminder(
            identifier: nil,
            title: "Reach out to Test Person",
            dueDate: Date(),
            notes: "Aim for this weekend"
        )
        XCTAssertFalse(identifier.isEmpty)
        let isCompleted = try await service.isCompleted(identifier: identifier)
        XCTAssertEqual(isCompleted, false)
    }

    func test_upsertReminder_updatesExistingReminderInPlace() async throws {
        let originalId = try service.upsertReminder(
            identifier: nil, title: "Reach out to Test Person", dueDate: Date(), notes: nil
        )
        let newDueDate = Date().addingTimeInterval(86400 * 7)
        let updatedId = try service.upsertReminder(
            identifier: originalId, title: "Reach out to Test Person", dueDate: newDueDate, notes: nil
        )
        XCTAssertEqual(originalId, updatedId) // same identifier, not a new reminder
    }

    func test_isCompleted_returnsNilForUnknownIdentifier() async throws {
        let result = try await service.isCompleted(identifier: "not-a-real-id")
        XCTAssertNil(result)
    }
}
