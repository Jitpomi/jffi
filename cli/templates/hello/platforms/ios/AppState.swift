import SwiftUI

@MainActor
class AppState: ObservableObject {
    @Published var greeting: String = ""
    let core: Core

    init() {
        let core = Core()
        self.core = core
        self.greeting = core.greeting()
    }
}
