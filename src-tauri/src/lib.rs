use std::fs;
use std::path::Path;
use std::process::Command;
use serde::{Deserialize, Serialize};
use image::{GenericImageView, imageops::FilterType, ImageFormat};

mod ffmpeg_manager;
use ffmpeg_manager::FFmpegManager;

#[derive(Debug, Serialize, Deserialize)]
struct FileInfo {
    size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CompressionResult {
    #[serde(rename = "compressedSize")]
    compressed_size: u64,
}

#[tauri::command]
async fn get_file_info(path: String) -> Result<FileInfo, String> {
    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
    Ok(FileInfo {
        size: metadata.len(),
    })
}

#[tauri::command]
async fn create_output_dir(path: String) -> Result<(), String> {
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn get_default_output_path() -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| {
        std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string())
    });
    Ok(format!("{}/Downloads/compressed", home))
}

#[tauri::command]
async fn open_directory(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
async fn compress_video(input_path: String, output_path: Option<String>) -> Result<CompressionResult, String> {
    // Ensure FFmpeg is available
    let ffmpeg_manager = FFmpegManager::new();
    let ffmpeg_path = ffmpeg_manager.ensure_ffmpeg().await?;
    
    let input = Path::new(&input_path);
    
    if !input.exists() {
        return Err("Input file does not exist".to_string());
    }
    
    let output_dir = if let Some(dir) = output_path {
        Path::new(&dir).to_path_buf()
    } else {
        input.parent().unwrap().join("compressed")
    };
    
    fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
    
    let file_name = input.file_stem().unwrap().to_str().unwrap();
    let extension = input.extension().unwrap_or_default().to_str().unwrap_or("mp4");
    let output_file = output_dir.join(format!("{}_compressed.{}", file_name, extension));
    
    let output = Command::new(&ffmpeg_path)
        .args(&[
            "-i", input_path.as_str(),
            "-c:v", "libx265",
            "-crf", "28",
            "-preset", "medium",
            "-c:a", "aac",
            "-b:a", "128k",
            "-movflags", "+faststart",
            "-y",
            output_file.to_str().unwrap(),
        ])
        .output();
    
    match output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                
                if stderr.contains("ffmpeg: not found") || stderr.contains("command not found") {
                    return Err("ffmpeg is not installed. Please install ffmpeg to compress videos.".to_string());
                }
                
                return Err(format!("Video compression failed: {}", stderr));
            }
            
            let metadata = fs::metadata(&output_file).map_err(|e| e.to_string())?;
            Ok(CompressionResult {
                compressed_size: metadata.len(),
            })
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err("ffmpeg is not installed. Please install ffmpeg to compress videos.".to_string())
            } else {
                Err(format!("Failed to run ffmpeg: {}", e))
            }
        }
    }
}

#[tauri::command]
async fn compress_image(input_path: String, output_path: Option<String>) -> Result<CompressionResult, String> {
    let input = Path::new(&input_path);
    
    if !input.exists() {
        return Err("Input file does not exist".to_string());
    }
    
    // Get original file size
    let original_size = fs::metadata(&input_path).map_err(|e| e.to_string())?.len();
    
    let img = image::open(&input_path).map_err(|e| e.to_string())?;
    
    let output_dir = if let Some(dir) = output_path {
        Path::new(&dir).to_path_buf()
    } else {
        input.parent().unwrap().join("compressed")
    };
    
    fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;
    
    let file_name = input.file_stem().unwrap().to_str().unwrap();
    let original_extension = input.extension().unwrap_or_default().to_str().unwrap_or("jpg");
    
    let (width, height) = img.dimensions();
    let max_dimension = 2048;
    
    // Resize only if image is larger than max dimension
    let resized = if width > max_dimension || height > max_dimension {
        let ratio = (max_dimension as f32) / (width.max(height) as f32);
        let new_width = (width as f32 * ratio) as u32;
        let new_height = (height as f32 * ratio) as u32;
        img.resize(new_width, new_height, FilterType::Lanczos3)
    } else {
        img
    };
    
    // For WebP and other already compressed formats, convert to JPEG if it would be smaller
    let (output_extension, output_format) = match original_extension.to_lowercase().as_str() {
        "webp" | "avif" => {
            // For already efficient formats, try JPEG and see if it's smaller
            ("jpg", ImageFormat::Jpeg)
        }
        "png" => {
            // PNG might be better kept as PNG if it has transparency
            if resized.color().has_alpha() {
                ("png", ImageFormat::Png)
            } else {
                ("jpg", ImageFormat::Jpeg)
            }
        }
        "gif" => ("gif", ImageFormat::Gif),
        "bmp" => ("jpg", ImageFormat::Jpeg),
        _ => ("jpg", ImageFormat::Jpeg),
    };
    
    let output_file = output_dir.join(format!("{}_compressed.{}", file_name, output_extension));
    
    // Save with quality optimization
    match output_format {
        ImageFormat::Jpeg => {
            // Use JPEG with quality 85 for good balance of quality and size
            let rgb_image = resized.to_rgb8();
            let mut jpeg_encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                std::fs::File::create(&output_file).map_err(|e| e.to_string())?,
                85
            );
            jpeg_encoder.encode_image(&rgb_image).map_err(|e| e.to_string())?;
        }
        ImageFormat::Png => {
            // Use PNG with compression
            let output = std::fs::File::create(&output_file).map_err(|e| e.to_string())?;
            let encoder = image::codecs::png::PngEncoder::new_with_quality(
                output,
                image::codecs::png::CompressionType::Best,
                image::codecs::png::FilterType::Adaptive
            );
            resized.write_with_encoder(encoder).map_err(|e| e.to_string())?;
        }
        _ => {
            resized.save(&output_file).map_err(|e| e.to_string())?;
        }
    }
    
    let compressed_size = fs::metadata(&output_file).map_err(|e| e.to_string())?.len();
    
    // If compressed is larger than original, just copy the original
    if compressed_size >= original_size {
        fs::copy(&input_path, &output_file).map_err(|e| e.to_string())?;
        let final_size = fs::metadata(&output_file).map_err(|e| e.to_string())?.len();
        Ok(CompressionResult {
            compressed_size: final_size,
        })
    } else {
        Ok(CompressionResult {
            compressed_size,
        })
    }
}

#[tauri::command]
async fn check_ffmpeg_status() -> Result<bool, String> {
    let ffmpeg_manager = FFmpegManager::new();
    Ok(ffmpeg_manager.is_ffmpeg_available() || ffmpeg_manager.is_system_ffmpeg_available())
}

#[tauri::command]
async fn download_ffmpeg() -> Result<String, String> {
    let ffmpeg_manager = FFmpegManager::new();
    ffmpeg_manager.ensure_ffmpeg().await?;
    Ok("FFmpeg downloaded successfully".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            get_file_info,
            create_output_dir,
            get_default_output_path,
            open_directory,
            compress_video,
            compress_image,
            check_ffmpeg_status,
            download_ffmpeg
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}