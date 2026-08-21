// Tests/FriendsTests/SchedulingEngineTests.swift
import XCTest
@testable import Friends

final class SchedulingEngineTests: XCTestCase {
    var calendar: Calendar!

    override func setUp() {
        super.setUp()
        calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "UTC")!
    }

    private func date(_ year: Int, _ month: Int, _ day: Int) -> Date {
        calendar.date(from: DateComponents(year: year, month: month, day: day))!
    }

    func test_matches_weekendsOnly_trueForSaturday() {
        let saturday = date(2026, 8, 22) // confirmed Saturday
        XCTAssertTrue(SchedulingEngine.matches(saturday, toggle: .weekendsOnly, calendar: calendar))
    }

    func test_matches_weekendsOnly_falseForWednesday() {
        let wednesday = date(2026, 8, 19) // confirmed Wednesday
        XCTAssertFalse(SchedulingEngine.matches(wednesday, toggle: .weekendsOnly, calendar: calendar))
    }

    func test_matches_allDays_alwaysTrue() {
        let wednesday = date(2026, 8, 19)
        XCTAssertTrue(SchedulingEngine.matches(wednesday, toggle: .allDays, calendar: calendar))
    }

    func test_matches_weekdaysOnly_falseForSunday() {
        let sunday = date(2026, 8, 23)
        XCTAssertFalse(SchedulingEngine.matches(sunday, toggle: .weekdaysOnly, calendar: calendar))
    }

    func test_snapToToggle_weekendsOnly_wednesdaySnapsToSaturday() {
        let wednesday = date(2026, 8, 19)
        let expected = date(2026, 8, 22) // next Saturday
        XCTAssertEqual(SchedulingEngine.snapToToggle(wednesday, toggle: .weekendsOnly, calendar: calendar), expected)
    }

    func test_snapToToggle_weekendsOnly_saturdayStaysSaturday() {
        let saturday = date(2026, 8, 22)
        XCTAssertEqual(SchedulingEngine.snapToToggle(saturday, toggle: .weekendsOnly, calendar: calendar), saturday)
    }

    func test_snapToToggle_weekdaysOnly_saturdaySnapsToMonday() {
        let saturday = date(2026, 8, 22)
        let expected = date(2026, 8, 24) // next Monday
        XCTAssertEqual(SchedulingEngine.snapToToggle(saturday, toggle: .weekdaysOnly, calendar: calendar), expected)
    }
}
