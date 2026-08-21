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

    func test_mondayOfWeek_forSaturday_returnsMondayOfSameWeek() {
        let saturday = date(2026, 8, 22)
        let expectedMonday = date(2026, 8, 17)
        XCTAssertEqual(SchedulingEngine.mondayOfWeek(containing: saturday, calendar: calendar), expectedMonday)
    }

    func test_mondayOfWeek_forSunday_returnsMondayOfSameWeek() {
        // Aug 23, 2026 is a Sunday. ISO week convention: the Monday that starts
        // that week is Aug 17 (the Sunday belongs to the week that started that Monday).
        let sunday = date(2026, 8, 23)
        let expectedMonday = date(2026, 8, 17)
        XCTAssertEqual(SchedulingEngine.mondayOfWeek(containing: sunday, calendar: calendar), expectedMonday)
    }

    func test_computeSchedule_allDaysToggle_weekdayTarget_notificationEqualsTarget() {
        let lastConnected = date(2026, 8, 1) // Saturday
        // 30 days later = Aug 31, a Monday
        let result = SchedulingEngine.computeSchedule(
            lastConnectedDate: lastConnected,
            frequencyDays: 30,
            toggle: .allDays,
            blockedRanges: [],
            calendar: calendar
        )
        let expectedTarget = date(2026, 8, 31)
        XCTAssertEqual(result.nextTargetDate, expectedTarget)
        XCTAssertEqual(result.notificationDate, expectedTarget) // weekday target: no shift
    }

    func test_computeSchedule_weekendsOnlyToggle_weekendTarget_notificationIsMonday() {
        let lastConnected = date(2026, 7, 23) // Thursday
        // 30 days later = Aug 22, a Saturday; weekendsOnly toggle keeps it there (already a weekend)
        let result = SchedulingEngine.computeSchedule(
            lastConnectedDate: lastConnected,
            frequencyDays: 30,
            toggle: .weekendsOnly,
            blockedRanges: [],
            calendar: calendar
        )
        let expectedTarget = date(2026, 8, 22) // Saturday
        let expectedNotification = date(2026, 8, 17) // Monday of that week
        XCTAssertEqual(result.nextTargetDate, expectedTarget)
        XCTAssertEqual(result.notificationDate, expectedNotification)
    }

    func test_computeSchedule_blockedRangeShiftsTarget() {
        let lastConnected = date(2026, 7, 23) // Thursday
        // Raw target Aug 22 (Saturday) falls inside this blocked range
        let blockedRange = BlockedRange(start: date(2026, 8, 20), end: date(2026, 8, 27))
        let result = SchedulingEngine.computeSchedule(
            lastConnectedDate: lastConnected,
            frequencyDays: 30,
            toggle: .weekendsOnly,
            blockedRanges: [blockedRange],
            calendar: calendar
        )
        let expectedTarget = date(2026, 8, 29) // next Saturday after the blocked range ends
        let expectedNotification = date(2026, 8, 24) // Monday of that week
        XCTAssertEqual(result.nextTargetDate, expectedTarget)
        XCTAssertEqual(result.notificationDate, expectedNotification)
    }

    func test_pushMissedSchedule_weekendsOnly_movesToNextWeekend() {
        let missedTarget = date(2026, 8, 22) // Saturday, now in the past and not completed
        let result = SchedulingEngine.pushMissedSchedule(
            currentTargetDate: missedTarget,
            toggle: .weekendsOnly,
            blockedRanges: [],
            calendar: calendar
        )
        let expectedTarget = date(2026, 8, 29) // next Saturday
        let expectedNotification = date(2026, 8, 24) // Monday of that week
        XCTAssertEqual(result.nextTargetDate, expectedTarget)
        XCTAssertEqual(result.notificationDate, expectedNotification)
    }

    func test_pushMissedSchedule_respectsBlockedRanges() {
        let missedTarget = date(2026, 8, 22) // Saturday
        let blockedRange = BlockedRange(start: date(2026, 8, 28), end: date(2026, 9, 4))
        let result = SchedulingEngine.pushMissedSchedule(
            currentTargetDate: missedTarget,
            toggle: .weekendsOnly,
            blockedRanges: [blockedRange],
            calendar: calendar
        )
        // Next Saturday after Aug 22 is Aug 29, which falls inside the blocked range,
        // so it must continue to the next Saturday/Sunday after Sep 4, which is Sep 5.
        let expectedTarget = date(2026, 9, 5)
        XCTAssertEqual(result.nextTargetDate, expectedTarget)
    }

    func test_pushMissedSchedule_allDays_movesToNextDay() {
        let missedTarget = date(2026, 8, 22) // any day
        let result = SchedulingEngine.pushMissedSchedule(
            currentTargetDate: missedTarget,
            toggle: .allDays,
            blockedRanges: [],
            calendar: calendar
        )
        let expectedTarget = date(2026, 8, 23) // next day
        XCTAssertEqual(result.nextTargetDate, expectedTarget)
    }
}
