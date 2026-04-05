import SwiftUI

class AppState: ObservableObject {
    @Published var items: [ItemViewModel] = []
    private let ffiApp: FfiApp
    
    init() {
        self.ffiApp = FfiApp()
        self.items = ffiApp.getItems()
    }
    
    func addItem(title: String) {
        let id = UUID().uuidString
        self.items = ffiApp.addItem(id: id, title: title)
    }
    
    func toggleItem(id: String) {
        self.items = ffiApp.toggleItem(id: id)
    }
    
    func deleteItem(id: String) {
        self.items = ffiApp.deleteItem(id: id)
    }
}

// Make ItemViewModel conform to Identifiable
extension ItemViewModel: Identifiable {}
