import SwiftUI

@main
struct {{name_pascal}}App: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(AppState())
        }
    }
}
