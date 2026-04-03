use colored::*;

pub fn list_platforms() {
    println!();
    println!("{}", "📱 Available Platforms".bright_green().bold());
    println!();
    
    let platforms = vec![
        ("ios", "iOS (iPhone/iPad)", "SwiftUI", "✅ Ready"),
        ("android", "Android", "Jetpack Compose", "🚧 Coming Soon"),
        ("macos", "macOS (Universal)", "SwiftUI", "🚧 Coming Soon"),
        ("macos-arm64", "macOS (Apple Silicon)", "SwiftUI", "🚧 Coming Soon"),
        ("macos-x64", "macOS (Intel)", "SwiftUI", "🚧 Coming Soon"),
        ("windows", "Windows (64-bit)", "WinUI 3", "🚧 Coming Soon"),
        ("windows-x64", "Windows (64-bit)", "WinUI 3", "🚧 Coming Soon"),
        ("windows-x86", "Windows (32-bit)", "WinUI 3", "🚧 Coming Soon"),
        ("linux", "Linux", "GTK 4", "🚧 Coming Soon"),
        ("web", "Web (WASM)", "HTML/JS", "🚧 Coming Soon"),
    ];
    
    println!("{:<15} {:<25} {:<20} {}", "Platform", "Description", "UI Framework", "Status");
    println!("{}", "─".repeat(80));
    
    for (platform, desc, ui, status) in platforms {
        println!(
            "{:<15} {:<25} {:<20} {}",
            platform.bright_cyan(),
            desc,
            ui.bright_yellow(),
            status
        );
    }
    
    println!();
    println!("Usage:");
    println!("  jffi new my-app --platforms ios,android");
    println!("  jffi new my-app --platforms multi  # All platforms");
    println!();
}
