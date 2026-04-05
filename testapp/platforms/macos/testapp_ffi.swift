import Foundation

class FfiApp {
    func getItems() -> [ItemViewModel] {
        // TODO: Call Rust FFI function
        return []
    }
    
    func addItem(id: String, title: String) -> [ItemViewModel] {
        // TODO: Call Rust FFI function
        return []
    }
    
    func toggleItem(id: String) -> [ItemViewModel] {
        // TODO: Call Rust FFI function
        return []
    }
    
    func deleteItem(id: String) -> [ItemViewModel] {
        // TODO: Call Rust FFI function
        return []
    }
}

struct ItemViewModel {
    let id: String
    let title: String
    let completed: Bool
}
