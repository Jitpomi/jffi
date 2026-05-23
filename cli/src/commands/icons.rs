use anyhow::{Context, Result};
use colored::*;
use image::imageops::FilterType;
use std::fs;
use std::path::Path;

use crate::config::Config;

pub fn generate_icons(config: &Config, platform: &str) -> Result<()> {
    let icons_config = match config.bundle.as_ref().and_then(|b| b.icons.as_ref()) {
        Some(config) if config.generate => config,
        _ => return Ok(()), // Opt-out or not configured
    };
    
    // Check for platform-specific icon source override in config
    let mut source_str = &icons_config.source;
    if let Some(bundle) = &config.bundle {
        if let Some(override_source) = match platform {
            "windows" => bundle.windows.as_ref().and_then(|w| w.icon.as_ref()),
            "macos" => bundle.macos.as_ref().and_then(|m| m.icon.as_ref()),
            "ios" => bundle.ios.as_ref().and_then(|i| i.icon.as_ref()),
            "android" => bundle.android.as_ref().and_then(|a| a.icon.as_ref()),
            "linux" => bundle.linux.as_ref().and_then(|l| l.icon.as_ref()),
            _ => None,
        } {
            source_str = override_source;
        }
    }
    
    let source_path = Path::new(source_str);
    if !source_path.exists() {
        println!("  {} Warning: Icon source not found at {}, skipping icon generation", "⚠".yellow(), source_str);
        return Ok(());
    }
    
    println!("  {} Generating icons from {}...", "→".bright_blue(), source_str);
    
    let img = image::open(source_path).context("Failed to open source icon image")?;
    
    match platform {
        "macos" => generate_apple_icons(&img, "platforms/macos/App/Assets.xcassets/AppIcon.appiconset", true)?,
        "ios" => generate_apple_icons(&img, "platforms/ios/App/Assets.xcassets/AppIcon.appiconset", false)?,
        "android" => generate_android_icons(&img)?,
        "windows" => generate_windows_icons(&img)?,
        "linux" => generate_linux_icons(&img)?,
        _ => {}
    }
    
    Ok(())
}

fn generate_apple_icons(img: &image::DynamicImage, dest_dir: &str, is_macos: bool) -> Result<()> {
    let dest_path = Path::new(dest_dir);
    fs::create_dir_all(dest_path)?;
    
    let mut contents = r#"{
  "images" : [
"#.to_string();

    let sizes = if is_macos {
        vec![
            (16, 1), (16, 2),
            (32, 1), (32, 2),
            (128, 1), (128, 2),
            (256, 1), (256, 2),
            (512, 1), (512, 2)
        ]
    } else {
        vec![
            (20, 2), (20, 3),
            (29, 2), (29, 3),
            (40, 2), (40, 3),
            (60, 2), (60, 3),
            (76, 2), (83, 2), // iPad
            (1024, 1) // App Store
        ]
    };

    let mut is_first = true;
    for (base_size, scale) in sizes {
        let actual_size = base_size * scale;
        let filename = format!("icon_{}x{}@{}.png", base_size, base_size, scale);
        let out_path = dest_path.join(&filename);
        
        let resized = img.resize_exact(actual_size, actual_size, FilterType::Lanczos3);
        resized.save(&out_path)?;
        
        if !is_first {
            contents.push_str(",\n");
        }
        is_first = false;
        
        let idiom = if is_macos { "mac" } else if base_size == 1024 { "ios-marketing" } else { "universal" };
        contents.push_str(&format!(
            "    {{\n      \"size\" : \"{}x{}\",\n      \"idiom\" : \"{}\",\n      \"filename\" : \"{}\",\n      \"scale\" : \"{}x\"\n    }}",
            base_size, base_size, idiom, filename, scale
        ));
    }

    contents.push_str(r#"
  ],
  "info" : {
    "version" : 1,
    "author" : "xcode"
  }
}"#);

    fs::write(dest_path.join("Contents.json"), contents)?;
    Ok(())
}

fn generate_android_icons(img: &image::DynamicImage) -> Result<()> {
    let base_dir = Path::new("platforms/android/app/src/main/res");
    
    let densities = vec![
        ("mipmap-mdpi", 48),
        ("mipmap-hdpi", 72),
        ("mipmap-xhdpi", 96),
        ("mipmap-xxhdpi", 144),
        ("mipmap-xxxhdpi", 192),
    ];
    
    for (dir_name, size) in densities {
        let dir_path = base_dir.join(dir_name);
        fs::create_dir_all(&dir_path)?;
        
        let out_path = dir_path.join("ic_launcher.png");
        let resized = img.resize_exact(size, size, FilterType::Lanczos3);
        resized.save(&out_path)?;
    }
    
    // Also generate play store icon
    let store_icon_dir = Path::new("platforms/android/fastlane/metadata/android/en-US/images");
    if store_icon_dir.exists() {
        let out_path = store_icon_dir.join("icon.png");
        let resized = img.resize_exact(512, 512, FilterType::Lanczos3);
        resized.save(&out_path)?;
    }
    
    Ok(())
}

fn generate_windows_icons(img: &image::DynamicImage) -> Result<()> {
    let dest_dir = Path::new("platforms/windows/Assets");
    if !dest_dir.exists() {
        fs::create_dir_all(dest_dir)?;
    }
    
    // Generate app.ico for the compiled WinExe icon and titlebar use
    let ico_path = dest_dir.join("app.ico");
    let resized_256 = img.resize_exact(256, 256, FilterType::Lanczos3);
    resized_256.save(&ico_path)?;

    let sizes = vec![
        ("Square44x44Logo.png", 44),
        ("Square44x44Logo.scale-200.png", 88),
        ("Square150x150Logo.png", 150),
        ("Square150x150Logo.scale-200.png", 300),
        ("StoreLogo.png", 50),
        ("StoreLogo.scale-200.png", 100),
        ("LockScreenLogo.scale-200.png", 48),
    ];
    
    for (filename, size) in sizes {
        let out_path = dest_dir.join(filename);
        let resized = img.resize_exact(size, size, FilterType::Lanczos3);
        resized.save(&out_path)?;
    }

    // Wide logo and Splash screen generation
    let splash_sizes = vec![
        ("SplashScreen.png", 620, 300, 300),
        ("SplashScreen.scale-200.png", 1240, 600, 600),
        ("Wide310x150Logo.scale-200.png", 620, 300, 300),
    ];

    for (filename, canvas_w, canvas_h, logo_size) in splash_sizes {
        let out_path = dest_dir.join(filename);
        let resized = img.resize_exact(logo_size, logo_size, FilterType::Lanczos3);
        let mut bg = image::RgbaImage::new(canvas_w, canvas_h);
        // Center the logo inside the canvas
        let x = i64::from((canvas_w - logo_size) / 2);
        let y = i64::from((canvas_h - logo_size) / 2);
        image::imageops::overlay(&mut bg, &resized, x, y);
        bg.save(&out_path)?;
    }
    
    Ok(())
}

fn generate_linux_icons(img: &image::DynamicImage) -> Result<()> {
    let dest_dir = Path::new("platforms/linux/data/icons/hicolor");
    
    let sizes = vec![16, 32, 48, 64, 128, 256, 512];
    
    for size in sizes {
        let dir_path = dest_dir.join(format!("{}x{}/apps", size, size));
        fs::create_dir_all(&dir_path)?;
        
        let out_path = dir_path.join("org.jffi.App.png");
        let resized = img.resize_exact(size, size, FilterType::Lanczos3);
        resized.save(&out_path)?;
    }
    
    Ok(())
}
