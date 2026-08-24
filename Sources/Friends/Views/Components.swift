// Sources/Friends/Views/Components.swift
// Small, reusable pieces of the "Friends" design system. Kept separate from
// the feature views so each view file stays focused on its own layout/logic,
// and so these primitives aren't duplicated across files.
import SwiftUI

/// A circular initial badge, colored deterministically per-name so a roster
/// reads as varied rather than a wall of identical rows.
struct AvatarBadge: View {
    let name: String
    var diameter: CGFloat = 30

    private var initial: String {
        name.trimmingCharacters(in: .whitespaces).first.map(String.init)?.uppercased() ?? "?"
    }

    var body: some View {
        Circle()
            .fill(FriendsTheme.avatarColor(for: name).gradient)
            .frame(width: diameter, height: diameter)
            .overlay {
                Text(initial)
                    .font(.system(size: diameter * 0.42, weight: .bold, design: .rounded))
                    .foregroundStyle(.white)
            }
    }
}

/// A small-caps section header, e.g. "DUE SOON" — used to break up the
/// dropdown and preference panes without resorting to plain `Divider()`s.
struct SectionLabel: View {
    let text: String
    var color: Color = FriendsTheme.inkSoft

    var body: some View {
        Text(text)
            .font(FriendsTheme.label())
            .tracking(1.1)
            .textCase(.uppercase)
            .foregroundStyle(color)
    }
}

/// A caption-style label placed above a field, replacing the default
/// leading-label `Form` layout with a more editorial, stacked one.
struct FieldLabel: View {
    let text: String

    var body: some View {
        Text(text)
            .font(FriendsTheme.label(10))
            .tracking(0.8)
            .textCase(.uppercase)
            .foregroundStyle(FriendsTheme.inkFaint)
    }
}

/// Wraps content in a warm "card" — the base visual unit of every view in
/// this app, replacing plain system `Form`/`List` backgrounds.
struct CardBackground: ViewModifier {
    var padding: CGFloat = 14

    func body(content: Content) -> some View {
        content
            .padding(padding)
            .background(FriendsTheme.paper, in: RoundedRectangle(cornerRadius: 12, style: .continuous))
            .overlay {
                RoundedRectangle(cornerRadius: 12, style: .continuous)
                    .strokeBorder(FriendsTheme.hairline, lineWidth: 1)
            }
    }
}

extension View {
    func friendsCard(padding: CGFloat = 14) -> some View {
        modifier(CardBackground(padding: padding))
    }
}

/// A filled, capsule-shaped primary action button (e.g. "Add Person").
struct PrimaryButtonStyle: ButtonStyle {
    var tint: Color = FriendsTheme.rust
    var isEnabled: Bool = true

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(FriendsTheme.body(13, weight: .semibold))
            .foregroundStyle(.white)
            .padding(.horizontal, 18)
            .padding(.vertical, 9)
            .background(
                Capsule().fill(isEnabled ? tint : tint.opacity(0.35))
            )
            .scaleEffect(configuration.isPressed ? 0.97 : 1)
            .shadow(color: tint.opacity(isEnabled ? 0.35 : 0), radius: 6, y: 3)
            .animation(.easeOut(duration: 0.12), value: configuration.isPressed)
    }
}

/// A quiet, text-only button for secondary actions (e.g. "Preferences…").
struct QuietButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(FriendsTheme.body(12, weight: .medium))
            .foregroundStyle(FriendsTheme.inkSoft)
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
            .background(
                Capsule().fill(FriendsTheme.ink.opacity(configuration.isPressed ? 0.10 : 0.05))
            )
    }
}

/// A horizontal row of capsule "pills," one per option — a warmer, more
/// tactile stand-in for `Picker`'s default segmented/menu styles.
struct PillPicker<Value: Hashable>: View {
    let options: [(label: String, value: Value)]
    @Binding var selection: Value

    var body: some View {
        HStack(spacing: 6) {
            ForEach(options, id: \.value) { option in
                let isSelected = option.value == selection
                Button {
                    selection = option.value
                } label: {
                    Text(option.label)
                        .font(FriendsTheme.body(12, weight: isSelected ? .semibold : .regular))
                        .foregroundStyle(isSelected ? .white : FriendsTheme.inkSoft)
                        .padding(.horizontal, 12)
                        .padding(.vertical, 6)
                        .background(
                            Capsule().fill(isSelected ? FriendsTheme.rust : FriendsTheme.ink.opacity(0.06))
                        )
                }
                .buttonStyle(.plain)
            }
        }
    }
}

/// A single row in the custom Preferences sidebar.
struct SidebarButton: View {
    let title: String
    let systemImage: String
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 9) {
                Image(systemName: systemImage)
                    .font(.system(size: 13, weight: .medium))
                    .frame(width: 18)
                Text(title)
                    .font(FriendsTheme.body(13, weight: isSelected ? .semibold : .regular))
                Spacer()
            }
            .foregroundStyle(isSelected ? .white : FriendsTheme.ink)
            .padding(.horizontal, 10)
            .padding(.vertical, 7)
            .background(
                RoundedRectangle(cornerRadius: 8, style: .continuous)
                    .fill(isSelected ? FriendsTheme.rust : Color.clear)
            )
        }
        .buttonStyle(.plain)
    }
}

/// A minimal, underline-style text field matching the app's editorial tone
/// (no boxed system `TextField` chrome).
struct FriendsTextFieldStyle: TextFieldStyle {
    func _body(configuration: TextField<Self._Label>) -> some View {
        configuration
            .font(FriendsTheme.body(14))
            .foregroundStyle(FriendsTheme.ink)
            .padding(.vertical, 6)
            .overlay(alignment: .bottom) {
                Rectangle().fill(FriendsTheme.hairline).frame(height: 1)
            }
            .textFieldStyle(.plain)
    }
}
