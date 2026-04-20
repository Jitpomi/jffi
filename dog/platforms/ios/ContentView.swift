import SwiftUI

struct ContentView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        VStack(spacing: 16) {
            // Text reads from @Published (reactive)
            Text(appState.greeting)
                .font(.title)
            
            // TextField writes to both
           TextField("Enter name", text:$appState.greeting)
                .onChange(of: appState.greeting) { newValue in
                    appState.core.setName(name: newValue)
                }
            
            
            Text(appState.core.getName())
                .font(.title)
            
            TextField("Enter name", text: Binding(
                get: { appState.core.getName() },
                set: { appState.core.setName(name: $0) }
            ))
            
        }
        .padding()
    }
}

#Preview {
    ContentView()
        .environmentObject(AppState())
}
