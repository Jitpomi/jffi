use anyhow::Result;
use colored::*;
use std::fs;
use std::path::PathBuf;

pub fn create_android_project(platforms_dir: &PathBuf, name: &str) -> Result<()> {
    let android_dir = platforms_dir.join("android");
    fs::create_dir_all(&android_dir)?;
    
    println!("  {} platforms/android/", "✓".green());
    
    // Create Android project structure
    create_gradle_files(&android_dir, name)?;
    create_gradle_wrapper(&android_dir)?;
    create_manifest(&android_dir, name)?;
    create_kotlin_sources(&android_dir, name)?;
    create_resources(&android_dir)?;
    
    Ok(())
}

fn create_gradle_files(dir: &PathBuf, name: &str) -> Result<()> {
    // build.gradle.kts (root)
    let root_gradle = r#"plugins {
    id("com.android.application") version "8.2.0" apply false
    id("org.jetbrains.kotlin.android") version "1.9.20" apply false
}
"#;
    fs::write(dir.join("build.gradle.kts"), root_gradle)?;
    
    // settings.gradle.kts
    let settings_gradle = format!(r#"pluginManagement {{
    repositories {{
        google()
        mavenCentral()
        gradlePluginPortal()
    }}
}}

dependencyResolutionManagement {{
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {{
        google()
        mavenCentral()
    }}
}}

rootProject.name = "{}"
include(":app")
"#, name);
    fs::write(dir.join("settings.gradle.kts"), settings_gradle)?;
    
    // app/build.gradle.kts
    let app_dir = dir.join("app");
    fs::create_dir_all(&app_dir)?;
    
    let package_name = format!("com.example.{}", name.replace("-", ""));
    let app_gradle = format!(r#"plugins {{
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}}

android {{
    namespace = "{}"
    compileSdk = 34

    defaultConfig {{
        applicationId = "{}"
        minSdk = 24
        targetSdk = 34
        versionCode = 1
        versionName = "1.0"
    }}

    buildTypes {{
        release {{
            isMinifyEnabled = false
        }}
    }}
    
    compileOptions {{
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }}
    
    kotlinOptions {{
        jvmTarget = "1.8"
        freeCompilerArgs += listOf("-Xjvm-default=all")
    }}
    
    buildFeatures {{
        compose = true
    }}
    
    composeOptions {{
        kotlinCompilerExtensionVersion = "1.5.4"
    }}
    
    sourceSets {{
        getByName("main") {{
            java.srcDirs("src/main/java")
        }}
    }}
}}

dependencies {{
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.7.0")
    implementation("androidx.activity:activity-compose:1.8.2")
    implementation(platform("androidx.compose:compose-bom:2024.01.00"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")
    
    // JNA for UniFFI bindings
    implementation("net.java.dev.jna:jna:5.13.0@aar")
}}
"#, package_name, package_name);
    fs::write(app_dir.join("build.gradle.kts"), app_gradle)?;
    
    // gradle.properties
    let gradle_props = r#"org.gradle.jvmargs=-Xmx2048m -Dfile.encoding=UTF-8
android.useAndroidX=true
android.enableJetifier=true
kotlin.code.style=official
"#;
    fs::write(dir.join("gradle.properties"), gradle_props)?;
    
    Ok(())
}

fn create_manifest(dir: &PathBuf, name: &str) -> Result<()> {
    let _package_name = format!("com.example.{}", name.replace("-", ""));
    let manifest_dir = dir.join("app/src/main");
    fs::create_dir_all(&manifest_dir)?;
    
    let manifest = r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <application
        android:allowBackup="true"
        android:label="@string/app_name"
        android:supportsRtl="true"
        android:theme="@style/Theme.App">
        <activity
            android:name=".MainActivity"
            android:exported="true"
            android:theme="@style/Theme.App">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"#;
    fs::write(manifest_dir.join("AndroidManifest.xml"), manifest)?;
    
    Ok(())
}

fn create_kotlin_sources(dir: &PathBuf, name: &str) -> Result<()> {
    let package_name = format!("com.example.{}", name.replace("-", ""));
    let package_path = package_name.replace(".", "/");
    let kotlin_dir = dir.join(format!("app/src/main/java/{}", package_path));
    fs::create_dir_all(&kotlin_dir)?;
    
    // MainActivity.kt
    let main_activity = format!(r#"package {}

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.outlined.Circle
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp

class MainActivity : ComponentActivity() {{
    override fun onCreate(savedInstanceState: Bundle?) {{
        super.onCreate(savedInstanceState)
        setContent {{
            MaterialTheme {{
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {{
                    TodoApp()
                }}
            }}
        }}
    }}
}}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TodoApp() {{
    var items by remember {{ mutableStateOf(listOf<TodoItem>()) }}
    var showDialog by remember {{ mutableStateOf(false) }}
    
    Scaffold(
        topBar = {{
            TopAppBar(
                title = {{ Text("Today") }},
                actions = {{
                    IconButton(onClick = {{ showDialog = true }}) {{
                        Icon(Icons.Default.Add, contentDescription = "Add Task")
                    }}
                }}
            )
        }}
    ) {{ padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp)
        ) {{
            // Stats Cards
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp)
            ) {{
                StatCard(
                    title = "Total",
                    count = items.size,
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.weight(1f)
                )
                StatCard(
                    title = "Active",
                    count = items.count {{ !it.completed }},
                    color = MaterialTheme.colorScheme.tertiary,
                    modifier = Modifier.weight(1f)
                )
                StatCard(
                    title = "Done",
                    count = items.count {{ it.completed }},
                    color = MaterialTheme.colorScheme.secondary,
                    modifier = Modifier.weight(1f)
                )
            }}
            
            Spacer(modifier = Modifier.height(24.dp))
            
            Text(
                text = "Tasks",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Bold
            )
            
            Spacer(modifier = Modifier.height(12.dp))
            
            if (items.isEmpty()) {{
                EmptyState()
            }} else {{
                LazyColumn(
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {{
                    items(items) {{ item ->
                        TaskRow(
                            item = item,
                            onToggle = {{
                                items = items.map {{
                                    if (it.id == item.id) it.copy(completed = !it.completed)
                                    else it
                                }}
                            }}
                        )
                    }}
                }}
            }}
        }}
    }}
    
    if (showDialog) {{
        AddTaskDialog(
            onDismiss = {{ showDialog = false }},
            onAdd = {{ title ->
                items = items + TodoItem(
                    id = items.size.toLong(),
                    title = title,
                    completed = false
                )
                showDialog = false
            }}
        )
    }}
}}

@Composable
fun StatCard(
    title: String,
    count: Int,
    color: androidx.compose.ui.graphics.Color,
    modifier: Modifier = Modifier
) {{
    Card(
        modifier = modifier,
        colors = CardDefaults.cardColors(
            containerColor = color.copy(alpha = 0.1f)
        )
    ) {{
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {{
            Text(
                text = count.toString(),
                style = MaterialTheme.typography.headlineMedium,
                fontWeight = FontWeight.Bold,
                color = color
            )
            Text(
                text = title,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }}
    }}
}}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TaskRow(
    item: TodoItem,
    onToggle: () -> Unit
) {{
    Card(
        modifier = Modifier.fillMaxWidth(),
        onClick = onToggle
    ) {{
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {{
            Icon(
                imageVector = if (item.completed) Icons.Filled.CheckCircle else Icons.Outlined.Circle,
                contentDescription = if (item.completed) "Completed" else "Not completed",
                tint = if (item.completed) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant
            )
            Spacer(modifier = Modifier.width(12.dp))
            Text(
                text = item.title,
                style = MaterialTheme.typography.bodyLarge,
                textDecoration = if (item.completed) TextDecoration.LineThrough else null,
                color = if (item.completed) MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.onSurface
            )
        }}
    }}
}}

@Composable
fun EmptyState() {{
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 48.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {{
        Icon(
            imageVector = Icons.Outlined.Circle,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f)
        )
        Spacer(modifier = Modifier.height(16.dp))
        Text(
            text = "No tasks yet",
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Text(
            text = "Click + to add your first task",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f)
        )
    }}
}}

@Composable
fun AddTaskDialog(
    onDismiss: () -> Unit,
    onAdd: (String) -> Unit
) {{
    var text by remember {{ mutableStateOf("") }}
    
    AlertDialog(
        onDismissRequest = onDismiss,
        title = {{ Text("New Task") }},
        text = {{
            OutlinedTextField(
                value = text,
                onValueChange = {{ text = it }},
                label = {{ Text("Task Name") }},
                singleLine = true
            )
        }},
        confirmButton = {{
            TextButton(
                onClick = {{
                    if (text.isNotBlank()) {{
                        onAdd(text)
                    }}
                }},
                enabled = text.isNotBlank()
            ) {{
                Text("Add")
            }}
        }},
        dismissButton = {{
            TextButton(onClick = onDismiss) {{
                Text("Cancel")
            }}
        }}
    )
}}

data class TodoItem(
    val id: Long,
    val title: String,
    val completed: Boolean
)
"#, package_name);
    fs::write(kotlin_dir.join("MainActivity.kt"), main_activity)?;
    
    Ok(())
}

fn create_resources(dir: &PathBuf) -> Result<()> {
    let res_dir = dir.join("app/src/main/res");
    
    // values/strings.xml
    let values_dir = res_dir.join("values");
    fs::create_dir_all(&values_dir)?;
    let strings = r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">Todo</string>
</resources>
"#;
    fs::write(values_dir.join("strings.xml"), strings)?;
    
    // values/themes.xml
    let themes = r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <style name="Theme.App" parent="android:Theme.Material.Light.NoActionBar" />
</resources>
"#;
    fs::write(values_dir.join("themes.xml"), themes)?;
    
    // Create mipmap directories (launcher icons)
    for dpi in &["mdpi", "hdpi", "xhdpi", "xxhdpi", "xxxhdpi"] {
        let mipmap_dir = res_dir.join(format!("mipmap-{}", dpi));
        fs::create_dir_all(&mipmap_dir)?;
    }
    
    Ok(())
}

fn create_gradle_wrapper(dir: &PathBuf) -> Result<()> {
    use std::process::Command;
    
    // Create gradle wrapper directory first
    let gradle_dir = dir.join("gradle/wrapper");
    fs::create_dir_all(&gradle_dir)?;
    
    let wrapper_jar = gradle_dir.join("gradle-wrapper.jar");
    
    println!("    Downloading Gradle wrapper...");
    
    let download_result = Command::new("curl")
        .args(&[
            "-L",
            "-o",
            wrapper_jar.to_str().unwrap(),
            "https://raw.githubusercontent.com/gradle/gradle/master/gradle/wrapper/gradle-wrapper.jar",
        ])
        .status();
    
    if download_result.is_err() || !download_result.unwrap().success() {
        anyhow::bail!("Failed to download Gradle wrapper. Please check your internet connection.");
    }
    
    // Create gradle-wrapper.properties
    let wrapper_props = r#"distributionBase=GRADLE_USER_HOME
distributionPath=wrapper/dists
distributionUrl=https\://services.gradle.org/distributions/gradle-8.2-bin.zip
networkTimeout=10000
validateDistributionUrl=true
zipStoreBase=GRADLE_USER_HOME
zipStorePath=wrapper/dists
"#;
    fs::write(gradle_dir.join("gradle-wrapper.properties"), wrapper_props)?;
    
    // Create gradlew script
    let gradlew = r#"#!/bin/sh
APP_HOME=$( cd "${APP_HOME:-./}" && pwd -P ) || exit
CLASSPATH=$APP_HOME/gradle/wrapper/gradle-wrapper.jar
exec java -classpath "$CLASSPATH" org.gradle.wrapper.GradleWrapperMain "$@"
"#;
    fs::write(dir.join("gradlew"), gradlew)?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(dir.join("gradlew"))?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(dir.join("gradlew"), perms)?;
    }
    
    // Create gradlew.bat
    let gradlew_bat = r#"@echo off
set CLASSPATH=%~dp0gradle\wrapper\gradle-wrapper.jar
java -classpath "%CLASSPATH%" org.gradle.wrapper.GradleWrapperMain %*
"#;
    fs::write(dir.join("gradlew.bat"), gradlew_bat)?;
    
    Ok(())
}
