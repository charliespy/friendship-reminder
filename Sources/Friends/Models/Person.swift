// Sources/Friends/Models/Person.swift
import Foundation
import SwiftData

@Model
final class Person {
    var name: String
    var frequencyDays: Int
    var lastConnectedDate: Date
    var nextTargetDate: Date
    var notificationDate: Date
    var reminderIdentifier: String?

    init(
        name: String,
        frequencyDays: Int,
        lastConnectedDate: Date,
        nextTargetDate: Date,
        notificationDate: Date,
        reminderIdentifier: String? = nil
    ) {
        self.name = name
        self.frequencyDays = frequencyDays
        self.lastConnectedDate = lastConnectedDate
        self.nextTargetDate = nextTargetDate
        self.notificationDate = notificationDate
        self.reminderIdentifier = reminderIdentifier
    }
}
