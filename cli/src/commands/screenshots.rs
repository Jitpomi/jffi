use anyhow::{Context, Result};
use colored::*;
use image::{imageops, GenericImageView, Rgba, RgbaImage};
use std::fs;
use std::path::Path;

pub fn generate_screenshots() -> Result<()> {
    let source_dir = Path::new("playstore-assets");
    let dest_dir = Path::new("platforms/apple-screenshots");
    if !source_dir.exists() {
        println!("{} Directory 'playstore-assets' not found.", "⚠".yellow());
        return Ok(());
    }
    fs::create_dir_all(dest_dir)?;

    let entries = fs::read_dir(source_dir)?;
    let mut found = false;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().unwrap_or_default() == "png" {
            found = true;
            let filename = path.file_name().unwrap().to_string_lossy();

            // Skip non-screenshot assets
            if filename == "app_icon.png" || filename == "feature_graphic.png" {
                continue;
            }

            println!(
                "  {} Processing screenshot {}...",
                "→".bright_blue(),
                filename
            );
            let img = image::open(&path).context("Failed to open screenshot")?;

            // Generate 6.5" iPhone (1284 x 2778)
            pad_and_save(
                &img,
                1284,
                2778,
                &dest_dir.join(format!("6.5_iphone_{}", filename)),
            )?;
            // Generate 5.5" iPhone (1242 x 2208)
            pad_and_save(
                &img,
                1242,
                2208,
                &dest_dir.join(format!("5.5_iphone_{}", filename)),
            )?;
            // Generate 12.9" iPad (2048 x 2732)
            pad_and_save(
                &img,
                2048,
                2732,
                &dest_dir.join(format!("12.9_ipad_{}", filename)),
            )?;
            // Generate Mac (2880 x 1800)
            pad_and_save(
                &img,
                2880,
                1800,
                &dest_dir.join(format!("mac_{}", filename)),
            )?;
        }
    }

    if found {
        println!(
            "  {} Apple screenshots generated in platforms/apple-screenshots",
            "✓".green()
        );
    } else {
        println!(
            "  {} No PNG screenshots found in playstore-assets/",
            "⚠".yellow()
        );
    }
    Ok(())
}

fn pad_and_save(
    img: &image::DynamicImage,
    target_w: u32,
    target_h: u32,
    out_path: &Path,
) -> Result<()> {
    let (mut tw, mut th) = (target_w, target_h);
    // Auto-detect landscape vs portrait
    let (iw, ih) = img.dimensions();
    if iw > ih && tw < th {
        // Source is landscape, target is portrait -> swap target
        std::mem::swap(&mut tw, &mut th);
    }

    let scale = f32::min(tw as f32 / iw as f32, th as f32 / ih as f32);
    let new_w = (iw as f32 * scale).round() as u32;
    let new_h = (ih as f32 * scale).round() as u32;

    let resized = img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);

    let mut bg = RgbaImage::from_pixel(tw, th, Rgba([0, 0, 0, 255]));
    let x_offset = (tw - new_w) / 2;
    let y_offset = (th - new_h) / 2;

    imageops::overlay(
        &mut bg,
        &resized.to_rgba8(),
        x_offset as i64,
        y_offset as i64,
    );

    // Convert to RGB to strip alpha channel for Apple validation
    let rgb_bg = image::DynamicImage::ImageRgba8(bg).into_rgb8();
    rgb_bg.save(out_path)?;
    Ok(())
}
