package com.example.rt

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import uniffi.rt_core.Core

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    HelloApp()
                }
            }
        }
    }
}

@Composable
fun HelloApp(initialGreeting: String = "Loading...") {
    var greeting by remember { mutableStateOf(initialGreeting) }
    var core: Core? by remember { mutableStateOf(null) }
    
    if (initialGreeting == "Loading...") {
        LaunchedEffect(Unit) {
            val newCore = Core()
            core = newCore
            greeting = newCore.greeting()
        }
    }
    
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Text(
            text = greeting,
            style = MaterialTheme.typography.headlineMedium
        )
        
        Spacer(modifier = Modifier.height(16.dp))
        
        Button(onClick = {
            core?.let { greeting = it.greeting() }
        }) {
            Text("Refresh")
        }
    }
}

@Preview(
    name = "Hello Screen",
    showBackground = true,
    backgroundColor = 0xFFF5F5F5
)
@Composable
fun HelloAppPreview() {
    HelloApp(initialGreeting = "Hello from JFFI")
}
