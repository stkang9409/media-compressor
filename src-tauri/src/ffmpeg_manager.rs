use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use dirs;
use reqwest;
use std::io::Write;

#[cfg(target_os = "windows")]
const FFMPEG_URL: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip";
#[cfg(target_os = "windows")]
const FFMPEG_EXECUTABLE: &str = "ffmpeg.exe";

#[cfg(target_os = "macos")]
const FFMPEG_URL: &str = "https://evermeet.cx/ffmpeg/getrelease/ffmpeg/zip";
#[cfg(target_os = "macos")]
const FFMPEG_EXECUTABLE: &str = "ffmpeg";

#[cfg(target_os = "linux")]
const FFMPEG_URL: &str = "https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz";
#[cfg(target_os = "linux")]
const FFMPEG_EXECUTABLE: &str = "ffmpeg";

pub struct FFmpegManager {
    ffmpeg_dir: PathBuf,
    ffmpeg_path: PathBuf,
}

impl FFmpegManager {
    pub fn new() -> Self {
        let app_data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("media-compressor");
        
        let ffmpeg_dir = app_data_dir.join("ffmpeg");
        let ffmpeg_path = ffmpeg_dir.join(FFMPEG_EXECUTABLE);
        
        Self {
            ffmpeg_dir,
            ffmpeg_path,
        }
    }
    
    #[allow(dead_code)]
    pub fn get_ffmpeg_path(&self) -> PathBuf {
        if self.is_ffmpeg_available() {
            self.ffmpeg_path.clone()
        } else if self.is_system_ffmpeg_available() {
            PathBuf::from("ffmpeg")
        } else {
            self.ffmpeg_path.clone()
        }
    }
    
    pub fn is_ffmpeg_available(&self) -> bool {
        self.ffmpeg_path.exists() && self.test_ffmpeg(&self.ffmpeg_path)
    }
    
    pub fn is_system_ffmpeg_available(&self) -> bool {
        self.test_ffmpeg(&PathBuf::from("ffmpeg"))
    }
    
    fn test_ffmpeg(&self, path: &Path) -> bool {
        Command::new(path)
            .arg("-version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    pub async fn ensure_ffmpeg(&self) -> Result<PathBuf, String> {
        if self.is_ffmpeg_available() {
            return Ok(self.ffmpeg_path.clone());
        }
        
        if self.is_system_ffmpeg_available() {
            return Ok(PathBuf::from("ffmpeg"));
        }
        
        self.download_ffmpeg().await?;
        
        if self.is_ffmpeg_available() {
            Ok(self.ffmpeg_path.clone())
        } else {
            Err("Failed to download and install FFmpeg".to_string())
        }
    }
    
    async fn download_ffmpeg(&self) -> Result<(), String> {
        fs::create_dir_all(&self.ffmpeg_dir)
            .map_err(|e| format!("Failed to create FFmpeg directory: {}", e))?;
        
        let temp_file = self.ffmpeg_dir.join("ffmpeg_temp.download");
        
        // Download FFmpeg
        let response = reqwest::get(FFMPEG_URL)
            .await
            .map_err(|e| format!("Failed to download FFmpeg: {}", e))?;
        
        let bytes = response.bytes()
            .await
            .map_err(|e| format!("Failed to read download: {}", e))?;
        
        let mut file = fs::File::create(&temp_file)
            .map_err(|e| format!("Failed to create temp file: {}", e))?;
        
        file.write_all(&bytes)
            .map_err(|e| format!("Failed to write temp file: {}", e))?;
        
        // Extract based on platform
        #[cfg(target_os = "windows")]
        self.extract_zip(&temp_file)?;
        
        #[cfg(target_os = "macos")]
        self.extract_zip(&temp_file)?;
        
        #[cfg(target_os = "linux")]
        self.extract_tar_xz(&temp_file)?;
        
        // Clean up temp file
        fs::remove_file(&temp_file).ok();
        
        // Make executable on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.ffmpeg_path)
                .map_err(|e| format!("Failed to get file metadata: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&self.ffmpeg_path, perms)
                .map_err(|e| format!("Failed to set permissions: {}", e))?;
        }
        
        Ok(())
    }
    
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    fn extract_zip(&self, archive_path: &Path) -> Result<(), String> {
        use zip::ZipArchive;
        
        let file = fs::File::open(archive_path)
            .map_err(|e| format!("Failed to open archive: {}", e))?;
        
        let mut archive = ZipArchive::new(file)
            .map_err(|e| format!("Failed to read archive: {}", e))?;
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("Failed to extract file: {}", e))?;
            
            let file_name = file.name();
            
            // Look for ffmpeg executable
            if file_name.ends_with(FFMPEG_EXECUTABLE) || file_name.ends_with("ffmpeg") {
                let mut outfile = fs::File::create(&self.ffmpeg_path)
                    .map_err(|e| format!("Failed to create ffmpeg file: {}", e))?;
                
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Failed to extract ffmpeg: {}", e))?;
                
                break;
            }
        }
        
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    fn extract_tar_xz(&self, archive_path: &Path) -> Result<(), String> {
        use flate2::read::GzDecoder;
        use tar::Archive;
        use std::process::Command;
        
        // First decompress xz to tar
        let tar_path = self.ffmpeg_dir.join("ffmpeg.tar");
        
        Command::new("xz")
            .args(&["-d", "-c"])
            .arg(archive_path)
            .output()
            .map_err(|e| format!("Failed to decompress xz: {}", e))
            .and_then(|output| {
                if output.status.success() {
                    fs::write(&tar_path, output.stdout)
                        .map_err(|e| format!("Failed to write tar file: {}", e))
                } else {
                    Err("Failed to decompress xz file".to_string())
                }
            })?;
        
        // Extract tar
        let tar_file = fs::File::open(&tar_path)
            .map_err(|e| format!("Failed to open tar file: {}", e))?;
        
        let mut archive = Archive::new(tar_file);
        
        for entry in archive.entries().map_err(|e| format!("Failed to read tar: {}", e))? {
            let mut entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path().map_err(|e| format!("Failed to get path: {}", e))?;
            
            if path.ends_with("ffmpeg") {
                entry.unpack(&self.ffmpeg_path)
                    .map_err(|e| format!("Failed to extract ffmpeg: {}", e))?;
                break;
            }
        }
        
        // Clean up
        fs::remove_file(&tar_path).ok();
        
        Ok(())
    }
}