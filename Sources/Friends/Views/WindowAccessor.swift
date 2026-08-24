// Sources/Friends/Views/WindowAccessor.swift
// SwiftUI's `Settings` scene doesn't reliably expose a resize grip via
// `.windowResizability` alone in practice — this grabs the hosting NSWindow
// so PreferencesView can force the AppKit style mask to include `.resizable`.
import SwiftUI
import AppKit

struct WindowAccessor: NSViewRepresentable {
    var configure: (NSWindow) -> Void

    func makeNSView(context: Context) -> NSView {
        let view = NSView()
        DispatchQueue.main.async { [weak view] in
            guard let window = view?.window else { return }
            configure(window)
        }
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        DispatchQueue.main.async { [weak nsView] in
            guard let window = nsView?.window else { return }
            configure(window)
        }
    }
}
