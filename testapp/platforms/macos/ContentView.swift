import SwiftUI

struct ContentView: View {
    @EnvironmentObject var appState: AppState
    @State private var showingAddItem = false
    
    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                // Header
                HStack {
                    Text("Today")
                        .font(.system(size: 28, weight: .bold))
                    Spacer()
                    Button(action: { showingAddItem = true }) {
                        Image(systemName: "plus.circle.fill")
                            .font(.title2)
                            .foregroundColor(.blue)
                    }
                    .buttonStyle(.plain)
                }
                .padding(.horizontal, 20)
                .padding(.top, 20)
                
                // Stats Cards
                HStack(spacing: 12) {
                    StatCard(
                        title: "Total",
                        count: appState.items.count,
                        color: .blue
                    )
                    StatCard(
                        title: "Active",
                        count: appState.items.filter { !$0.completed }.count,
                        color: .orange
                    )
                    StatCard(
                        title: "Done",
                        count: appState.items.filter { $0.completed }.count,
                        color: .green
                    )
                }
                .padding(.horizontal, 20)
                
                // Tasks
                VStack(alignment: .leading, spacing: 12) {
                    Text("Tasks")
                        .font(.title3)
                        .fontWeight(.semibold)
                        .padding(.horizontal, 20)
                    
                    if appState.items.isEmpty {
                        EmptyStateView()
                    } else {
                        VStack(spacing: 8) {
                            ForEach(appState.items) { item in
                                TaskRow(item: item) {
                                    withAnimation(.spring(response: 0.3)) {
                                        appState.toggleItem(id: item.id)
                                    }
                                }
                            }
                        }
                        .padding(.horizontal, 20)
                    }
                }
            }
            .padding(.bottom, 20)
        }
        .background(Color(NSColor.windowBackgroundColor))
        .frame(minWidth: 600, minHeight: 450)
        .sheet(isPresented: $showingAddItem) {
            AddItemView(isPresented: $showingAddItem)
                .environmentObject(appState)
        }
    }
}

struct StatCard: View {
    let title: String
    let count: Int
    let color: Color
    
    var body: some View {
        VStack(spacing: 8) {
            Text("\(count)")
                .font(.system(size: 28, weight: .bold))
                .foregroundColor(color)
            Text(title)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 16)
        .background(
            RoundedRectangle(cornerRadius: 16)
                .fill(Color(NSColor.controlBackgroundColor))
                .shadow(color: color.opacity(0.1), radius: 8, x: 0, y: 4)
        )
    }
}

struct TaskRow: View {
    let item: ItemViewModel
    let onToggle: () -> Void
    
    var body: some View {
        Button(action: onToggle) {
            HStack(spacing: 16) {
                ZStack {
                    Circle()
                        .stroke(item.completed ? Color.green : Color.gray.opacity(0.3), lineWidth: 2)
                        .frame(width: 24, height: 24)
                    
                    if item.completed {
                        Image(systemName: "checkmark")
                            .font(.system(size: 12, weight: .bold))
                            .foregroundColor(.green)
                    }
                }
                
                Text(item.title)
                    .font(.body)
                    .strikethrough(item.completed)
                    .foregroundColor(item.completed ? .secondary : .primary)
                
                Spacer()
            }
            .padding()
            .background(
                RoundedRectangle(cornerRadius: 12)
                    .fill(Color(NSColor.controlBackgroundColor))
            )
            .opacity(item.completed ? 0.6 : 1.0)
        }
        .buttonStyle(.plain)
    }
}

struct EmptyStateView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "checkmark.circle")
                .font(.system(size: 60))
                .foregroundColor(.gray.opacity(0.3))
            
            Text("No tasks yet")
                .font(.title3)
                .fontWeight(.medium)
                .foregroundColor(.secondary)
            
            Text("Click + to add your first task")
                .font(.subheadline)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 60)
    }
}

struct AddItemView: View {
    @EnvironmentObject var appState: AppState
    @Binding var isPresented: Bool
    @State private var title = ""
    @FocusState private var isFocused: Bool
    
    var body: some View {
        VStack(spacing: 20) {
            // Title
            Text("New Task")
                .font(.title2)
                .fontWeight(.semibold)
            
            // Input field
            VStack(alignment: .leading, spacing: 8) {
                Text("Task Name")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                
                TextField("Enter task name", text: $title)
                    .textFieldStyle(.roundedBorder)
                    .focused($isFocused)
                    .onSubmit {
                        if !title.isEmpty {
                            appState.addItem(title: title)
                            isPresented = false
                        }
                    }
            }
            
            // Buttons
            HStack(spacing: 12) {
                Button("Cancel") {
                    isPresented = false
                }
                .keyboardShortcut(.cancelAction)
                .controlSize(.large)
                
                Button("Add Task") {
                    appState.addItem(title: title)
                    isPresented = false
                }
                .keyboardShortcut(.defaultAction)
                .disabled(title.isEmpty)
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
            }
        }
        .padding(24)
        .frame(width: 400)
        .onAppear {
            isFocused = true
        }
    }
}

#Preview {
    ContentView()
        .environmentObject(AppState())
}
