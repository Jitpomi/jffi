package com.example.cat

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.platform.LocalInspectionMode
import androidx.compose.ui.unit.dp
import uniffi.cat_core.Core
import uniffi.cat_core.CoreInterface

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
    // Avoid instantiating the native Core in Preview, as it relies on native libraries
    val isPreview = LocalInspectionMode.current
    val core = remember {
        if (isPreview) {
            object : CoreInterface {
                override fun greeting(): String = "Hello from Preview"
                override fun setName(name: String) {}
            }
        } else {
            Core()
        }
    }
    var greeting by remember { mutableStateOf(initialGreeting) }
    
    // Initialize Core when app starts (only in real app, not preview)
    if (initialGreeting == "Loading...") {
        LaunchedEffect(Unit) {
            greeting = core.greeting()
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
           core.setName("Rust by JFFI")
            greeting = core.greeting()
        }) {
            Text("Change")
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
    MaterialTheme {
        HelloApp(initialGreeting = "Hello from JFFI")
    }
}
