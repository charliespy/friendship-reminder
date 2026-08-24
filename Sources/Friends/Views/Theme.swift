// Sources/Friends/Views/Theme.swift
// Shared design tokens (color, type) for Friends' warm "address book" aesthetic.
// Centralizing these keeps every view visually consistent without duplicating
// raw color/font values across files.
import SwiftUI

enum FriendsTheme {
    // MARK: Palette — warm parchment + terracotta, deliberately avoiding the
    // generic blue-accent-on-white look most utility apps default to.
    static let cream = Color(red: 0.976, green: 0.960, blue: 0.925)
    static let paper = Color(red: 0.995, green: 0.987, blue: 0.968)
    static let ink = Color(red: 0.145, green: 0.115, blue: 0.095)
    static let inkSoft = ink.opacity(0.58)
    static let inkFaint = ink.opacity(0.34)
    static let hairline = ink.opacity(0.10)

    static let rust = Color(red: 0.718, green: 0.318, blue: 0.204)
    static let amber = Color(red: 0.831, green: 0.573, blue: 0.208)
    static let sage = Color(red: 0.404, green: 0.502, blue: 0.376)
    static let clay = Color(red: 0.616, green: 0.396, blue: 0.353)

    /// Rotating palette for avatar badges so a roster of names reads as
    /// varied and alive rather than one flat accent color repeated everywhere.
    static let avatarPalette: [Color] = [rust, amber, sage, clay]

    static func avatarColor(for name: String) -> Color {
        let index = abs(name.hashValue) % avatarPalette.count
        return avatarPalette[index]
    }

    // MARK: Type — serif for the app's "wordmark" moments, rounded system
    // font everywhere else for a friendly, human tone. Both are built-in
    // system font designs, so no bundled font resources are needed.
    static let wordmark = Font.system(size: 19, weight: .semibold, design: .serif)
    static let sectionTitle = Font.system(size: 16, weight: .semibold, design: .serif)

    static func label(_ size: CGFloat = 11) -> Font {
        .system(size: size, weight: .bold, design: .rounded)
    }

    static func body(_ size: CGFloat = 13, weight: Font.Weight = .regular) -> Font {
        .system(size: size, weight: weight, design: .rounded)
    }
}
