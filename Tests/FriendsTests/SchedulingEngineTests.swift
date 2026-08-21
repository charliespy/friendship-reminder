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

    func test_avoidBlockedRanges_dateInsideRange_movesToNextValidDayAfterRange() {
        let saturday = date(2026, 8, 22) // inside the blocked range below
        let blockedRange = BlockedRange(start: date(2026, 8, 20), end: date(2026, 8, 27))
        // next Saturday/Sunday strictly after Aug 27 is Aug 29
        let expected = date(2026, 8, 29)
        let result = SchedulingEngine.avoidBlockedRanges(
            saturday, ranges: [blockedRange], toggle: .weekendsOnly, calendar: calendar
        )
        XCTAssertEqual(result, expected)
    }

    func test_avoidBlockedRanges_dateOutsideAnyRange_unchanged() {
        let saturday = date(2026, 8, 22)
        let unrelatedRange = BlockedRange(start: date(2026, 9, 1), end: date(2026, 9, 5))
        let result = SchedulingEngine.avoidBlockedRanges(
            saturday, ranges: [unrelatedRange], toggle: .weekendsOnly, calendar: calendar
        )
        XCTAssertEqual(result, saturday)
    }

    func test_avoidBlockedRanges_multipleOverlappingRanges_skipsAllOfThem() {
        let saturday = date(2026, 8, 22)
        let rangeA = BlockedRange(start: date(2026, 8, 20), end: date(2026, 8, 27))
        let rangeB = BlockedRange(start: date(2026, 8, 28), end: date(2026, 8, 30))
        // After rangeA ends Aug 27, next weekend day Aug 29 falls inside rangeB (through Aug 30),
        // so it must continue searching past rangeB too. Next Saturday/Sunday after Aug 30 is Sep 5.
        let expected = date(2026, 9, 5)
        let result = SchedulingEngine.avoidBlockedRanges(
            saturday, ranges: [rangeA, rangeB], toggle: .weekendsOnly, calendar: calendar
        )
        XCTAssertEqual(result, expected)
    }
}
