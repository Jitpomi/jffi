# JFFI Architecture Principles

## Golden Rule

**Rust Core = Pure Computation Engine. Platform Layer = UI State Owner.**

JFFI bridges Rust's performance and safety with platform-native UI frameworks. The architecture is intentionally split:

- **Rust Core** (`core/`): Business logic, heavy computation, data transformation, FFI-safe APIs
- **Platform Layer** (`platforms/<platform>/`): UI state, reactive frameworks, user interactions

## Correct Pattern

### Rust ( Stateless )

```rust
#[derive(uniffi::Object)]
pub struct Core {
    // NO UI state here. NO Mutex<bool>, Mutex<Vec<Answer>>, etc.
}

#[uniffi::export]
impl Core {
    #[uniffi::constructor]
    pub fn new() -> Self { Self }

    // Pure function: input -> output
    pub fn greeting(&self) -> String {
        "Hello from JFFI".to_string()
    }

    // Heavy computation stays in Rust
    pub fn process_data(&self, input: String) -> String {
        format!("Processed: {}", input)
    }
}
```

### Kotlin ( UI State Owner )

```kotlin
class AppViewModel : ViewModel() {
    private val core = Core()

    // UI state lives HERE, not in Rust
    private val _uiState = MutableStateFlow(
        AppUiState(greeting = core.greeting())
    )
    val uiState: StateFlow<AppUiState> = _uiState.asStateFlow()

    fun toggleRefreshed() {
        // Zero FFI calls for UI mutations
        _uiState.update { it.copy(refreshed = !it.refreshed) }
    }

    fun submitData(text: String) {
        // ONE FFI call for computation, then update local state
        val result = core.process_data(text)
        _uiState.update { it.copy(result = result) }
    }
}
```

## Anti-Patterns ( Do Not Do This )

### ❌ Anti-Pattern 1: UI State in Rust

**What users try:**

```rust
pub struct Core {
    refreshed: Mutex<bool>,  // DON'T
    answers: Mutex<Vec<Answer>>, // DON'T
}

#[uniffi::export]
impl Core {
    pub fn toggle_refreshed(&self) -> bool {
        let mut r = self.refreshed.lock().unwrap();
        *r = !*r;
        *r
    }

    pub fn get_refreshed(&self) -> bool {
        *self.refreshed.lock().unwrap()
    }
}
```

**Why it breaks:**

| Scenario | Problem |
|---|---|
| Rapid toggles | Mutex contention across FFI, UI jank |
| Compose recomposition | Every recompose locks Rust mutex |
| Multiple platforms | iOS + Android + Web all compete for same `Mutex` |
| Async Rust | `lock().unwrap()` panics across await points |
| Debugging | "Where is `refreshed` true?" → hunt across FFI boundary |

**What users think:** *"Two-way binding between Rust and Kotlin"*
**What actually happens:** *Kotlin asks Rust to mutate, then asks Rust what happened — 3 FFI calls for a boolean toggle.*

### ❌ Anti-Pattern 2: Generic Mutex Helpers Exposed to Users

**What users try:**

```rust
fn with_state<T, R, F>(state: &Mutex<T>, f: F) -> R
where F: FnOnce(&mut T) -> R {
    let mut guard = state.lock().unwrap();
    f(&mut *guard)
}
```

**Why it breaks:**
- Exposes locking as a public API pattern
- Users copy-paste it for every UI field
- Creates invisible coupling: "my UI state is in Rust, accessed via generic lock helper"
- Eventually becomes ad-hoc distributed state manager over FFI

### ❌ Anti-Pattern 3: Round-Trip State Sync

**What users try:**

```kotlin
// Kotlin calls Rust, Rust mutates, Kotlin asks Rust for result
fun onClick() {
    core.toggleRefreshed()           // FFI call 1: mutate
    val newValue = core.getRefreshed() // FFI call 2: read back
    _uiState.update { it.copy(refreshed = newValue) }
}
```

**Why it breaks:**
- **2–3 FFI crossings per UI action**
- Race condition: what if Compose recomposes between `toggle()` and `get()`?
- No single source of truth — state lives in two places

## When Rust State IS Appropriate

Rust **can** hold state, but only for:

| Valid Use | Invalid Use |
|---|---|
| Database connection pool | `Mutex<bool> isExpanded` |
| File handle / cache | `Mutex<Vec<UiItem>>` |
| Crypto key material | `Mutex<String> currentScreen` |
| Background worker state | `Mutex<i32> clickCount` |

**Rule of thumb:** If the state is observed by Compose/SwiftUI/Flutter reactive framework, it belongs in the platform ViewModel/`@State`.

## Performance Numbers

| Operation | In-Platform (Kotlin) | FFI Round-Trip |
|---|---|---|
| Boolean toggle | ~1ns | ~50–200μs |
| `StateFlow.update {}` | ~100ns | +2× FFI penalty |
| List sort (1000 items) | ~1ms | Rust wins: ~100μs |

**Conclusion:** UI mutations in Kotlin, heavy compute in Rust.

## Platform-Specific State Ownership

| Platform | UI State Container |
|---|---|
| Android (Compose) | `ViewModel` + `StateFlow` |
| iOS (SwiftUI) | `ObservableObject` / `@State` |
| macOS (SwiftUI) | `ObservableObject` / `@State` |
| Web (React) | `useState` / `useReducer` |
| Windows (WinUI) | `ObservableObject` / `INotifyPropertyChanged` |
| Linux (GTK) | `gio::Property` / manual signals |

## Debugging Checklist

If your JFFI app feels sluggish or has race conditions:

1. [ ] Is UI state in Kotlin/Swift/React, or Rust?
2. [ ] Does a button click trigger more than 1 FFI call?
3. [ ] Are you calling Rust getters from `@Composable` / `body` / `render()`?
4. [ ] Do you have `Mutex<T>` where `T` is a UI concept?

If any answer is "yes," move that state to the platform layer.
