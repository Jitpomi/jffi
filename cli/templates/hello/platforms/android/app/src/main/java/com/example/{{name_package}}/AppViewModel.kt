package com.example.{{name_package}}

import androidx.lifecycle.ViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import uniffi.{{name_snake}}_core.Core

data class AppUiState(
    val greeting: String = ""
)

class AppViewModel : ViewModel() {
    private val core = Core()

    private val _uiState = MutableStateFlow(
        AppUiState(
            greeting = core.greeting()
        )
    )

    val uiState: StateFlow<AppUiState> = _uiState.asStateFlow()

    fun refreshGreeting() {
        _uiState.update { currentState ->
            currentState.copy(
                greeting = core.greeting()
            )
        }
    }

    override fun onCleared() {
        super.onCleared()
        // Clean up core resources if needed
    }
}
