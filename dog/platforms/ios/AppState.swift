import SwiftUI

// Pattern: @Published shadows Rust state for SwiftUI reactivity
// Changes flow: TextField → updateName() → Rust Core → @Published → UI update
class AppState: ObservableObject {
    @Published var greeting: String = ""
    let core: Core

    init() {
        let core = Core()
        self.core = core
        self.greeting = core.getName()
    }
 
}
