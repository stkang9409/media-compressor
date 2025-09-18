import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import "./App.css";

interface FileItem {
  path: string;
  name: string;
  size: number;
  type: 'video' | 'image';
  status: 'pending' | 'processing' | 'completed' | 'error';
  progress: number;
  compressedSize?: number;
}

function App() {
  const [files, setFiles] = useState<FileItem[]>([]);
  const [isDragging, setIsDragging] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [outputPath, setOutputPath] = useState<string>("");
  const [defaultOutputPath, setDefaultOutputPath] = useState<string>("");
  const [ffmpegAvailable, setFfmpegAvailable] = useState<boolean | null>(null);
  const [downloadingFfmpeg, setDownloadingFfmpeg] = useState(false);

  // Setup default output directory and check FFmpeg
  useEffect(() => {
    const setupApp = async () => {
      try {
        // Get default path
        const defaultPath = await invoke<string>('get_default_output_path');
        setDefaultOutputPath(defaultPath);
        setOutputPath(defaultPath);
        
        // Check FFmpeg status
        const ffmpegStatus = await invoke<boolean>('check_ffmpeg_status');
        setFfmpegAvailable(ffmpegStatus);
      } catch (error) {
        console.error('Error during setup:', error);
        // Fallback to a simple default path
        const fallbackPath = "~/Downloads/compressed";
        setDefaultOutputPath(fallbackPath);
        setOutputPath(fallbackPath);
        setFfmpegAvailable(false);
      }
    };
    
    setupApp();
  }, []);

  // Setup Tauri file drop listener
  useEffect(() => {
    let unlistenFileDrop: (() => void) | null = null;
    
    const setupListeners = async () => {
      console.log('Setting up file drop listeners...');
      const appWindow = getCurrentWebviewWindow();
      
      // Listen for file drop event
      unlistenFileDrop = await appWindow.onDragDropEvent((event) => {
        console.log('Drag drop event:', event);
        
        if (event.payload.type === 'drop') {
          console.log('Files dropped:', event.payload.paths);
          processFilePaths(event.payload.paths);
          setIsDragging(false);
        } else if (event.payload.type === 'over') {
          console.log('Dragging over window');
          setIsDragging(true);
        } else if (event.payload.type === 'leave') {
          console.log('Drag left window');
          setIsDragging(false);
        }
      });
    };
    
    setupListeners().catch(console.error);
    
    return () => {
      if (unlistenFileDrop) {
        unlistenFileDrop();
      }
    };
  }, []);

  const processFilePaths = useCallback(async (paths: string[]) => {
    console.log('Processing paths:', paths);
    const allPaths: string[] = [];
    
    // Process each path - could be file or directory
    for (const path of paths) {
      try {
        // Try to get directory files first
        const dirFiles = await invoke<string[]>('get_directory_files', { dirPath: path });
        console.log(`Found ${dirFiles.length} files in directory: ${path}`);
        allPaths.push(...dirFiles);
      } catch (error) {
        // If it fails, it's probably a file, not a directory
        console.log(`Path ${path} is not a directory or error occurred:`, error);
        allPaths.push(path);
      }
    }
    
    const newFiles: FileItem[] = [];
    
    for (const path of allPaths) {
      const name = path.split('/').pop() || path.split('\\').pop() || '';
      const extension = name.split('.').pop()?.toLowerCase();
      const isVideo = ['mp4', 'avi', 'mov', 'mkv', 'wmv', 'flv'].includes(extension || '');
      const isImage = ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp'].includes(extension || '');
      
      if (isVideo || isImage) {
        try {
          const fileInfo = await invoke<{ size: number }>('get_file_info', { path });
          
          newFiles.push({
            path,
            name,
            size: fileInfo.size,
            type: isVideo ? 'video' : 'image',
            status: 'pending',
            progress: 0
          });
        } catch (error) {
          console.error(`Error getting file info for ${path}:`, error);
        }
      }
    }
    
    if (newFiles.length > 0) {
      setFiles(prev => {
        // Check for duplicates when adding
        const existingPaths = new Set(prev.map(f => f.path));
        const uniqueNewFiles = newFiles.filter(f => !existingPaths.has(f.path));
        console.log(`Adding ${uniqueNewFiles.length} unique files out of ${newFiles.length} total`);
        return [...prev, ...uniqueNewFiles];
      });
    }
  }, []);

  const selectFiles = async () => {
    try {
      const selected = await open({
        multiple: true,
        filters: [{
          name: 'Media',
          extensions: ['mp4', 'avi', 'mov', 'mkv', 'jpg', 'jpeg', 'png', 'gif']
        }]
      });

      if (selected) {
        const paths = Array.isArray(selected) ? selected : [selected];
        await processFilePaths(paths);
      }
    } catch (error) {
      console.error('Error selecting files:', error);
    }
  };

  const selectOutputDirectory = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: outputPath || defaultOutputPath
      });

      if (selected && typeof selected === 'string') {
        setOutputPath(selected);
      }
    } catch (error) {
      console.error('Error selecting directory:', error);
    }
  };

  const openOutputDirectory = async () => {
    try {
      const pathToOpen = outputPath || defaultOutputPath;
      if (pathToOpen) {
        // Create directory if it doesn't exist
        await invoke('create_output_dir', { path: pathToOpen });
        // Open the directory in file explorer using Rust backend
        await invoke('open_directory', { path: pathToOpen });
      }
    } catch (error) {
      console.error('Error opening directory:', error);
    }
  };

  const downloadFfmpeg = async () => {
    setDownloadingFfmpeg(true);
    try {
      await invoke('download_ffmpeg');
      setFfmpegAvailable(true);
    } catch (error) {
      console.error('Error downloading FFmpeg:', error);
      alert('Failed to download FFmpeg: ' + error);
    } finally {
      setDownloadingFfmpeg(false);
    }
  };

  const compressFiles = async () => {
    if (files.length === 0) return;
    
    // Check if we have video files and FFmpeg is not available
    const hasVideoFiles = files.some(f => f.type === 'video');
    if (hasVideoFiles && ffmpegAvailable === false) {
      const shouldDownload = confirm('FFmpeg is required for video compression. Would you like to download it automatically?');
      if (shouldDownload) {
        await downloadFfmpeg();
        if (!ffmpegAvailable) return;
      } else {
        alert('Video compression requires FFmpeg. Please install it manually or allow automatic download.');
        return;
      }
    }
    
    setIsProcessing(true);
    
    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      if (file.status === 'completed') continue;
      
      setFiles(prev => {
        const updated = [...prev];
        updated[i] = { ...updated[i], status: 'processing' };
        return updated;
      });
      
      try {
        const result = await invoke<{ compressedSize: number }>(
          file.type === 'video' ? 'compress_video' : 'compress_image',
          { 
            inputPath: file.path,
            outputPath: outputPath || undefined
          }
        );
        
        setFiles(prev => {
          const updated = [...prev];
          updated[i] = {
            ...updated[i],
            status: 'completed',
            progress: 100,
            compressedSize: result.compressedSize
          };
          return updated;
        });
      } catch (error) {
        setFiles(prev => {
          const updated = [...prev];
          updated[i] = { ...updated[i], status: 'error' };
          return updated;
        });
        console.error(`Failed to compress ${file.name}:`, error);
      }
    }
    
    setIsProcessing(false);
  };

  const clearFiles = () => {
    setFiles([]);
  };

  const formatFileSize = (bytes: number) => {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i];
  };

  return (
    <main className="container">
      <h1>Media Compressor</h1>
      <p className="subtitle">Drag and drop videos, images, or folders to compress</p>

      <div
        className={`drop-zone ${isDragging ? 'dragging' : ''}`}
        onClick={selectFiles}
      >
        <svg className="upload-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
        </svg>
        <p>Drop files or folders here, or click to select</p>
        <p className="file-types">Supports MP4, AVI, MOV, MKV, JPG, PNG, GIF, WebP</p>
      </div>

      {files.length > 0 && (
        <div className="file-list">
          <div className="file-list-header">
            <h3>Files ({files.length})</h3>
            <button onClick={clearFiles} className="clear-btn">Clear All</button>
          </div>
          
          {files.map((file, index) => (
            <div key={index} className="file-item">
              <div className="file-info">
                <span className={`file-type ${file.type}`}>
                  {file.type === 'video' ? '🎬' : '🖼️'}
                </span>
                <div className="file-details">
                  <p className="file-name">{file.name}</p>
                  <div className="file-sizes">
                    <span>Original: {formatFileSize(file.size)}</span>
                    {file.compressedSize && (
                      <span className="compressed-size">
                        → Compressed: {formatFileSize(file.compressedSize)}
                        ({Math.round((1 - file.compressedSize / file.size) * 100)}% smaller)
                      </span>
                    )}
                  </div>
                </div>
              </div>
              <div className="file-status">
                {file.status === 'pending' && <span className="status-pending">Waiting</span>}
                {file.status === 'processing' && (
                  <div className="progress-bar">
                    <div className="progress-fill" style={{ width: `${file.progress}%` }} />
                  </div>
                )}
                {file.status === 'completed' && <span className="status-completed">✓ Done</span>}
                {file.status === 'error' && <span className="status-error">✗ Error</span>}
              </div>
            </div>
          ))}
        </div>
      )}

      {files.length > 0 && (
        <div className="controls">
          {ffmpegAvailable === false && files.some(f => f.type === 'video') && (
            <div className="ffmpeg-notice">
              <span>⚠️ FFmpeg is required for video compression</span>
              <button 
                onClick={downloadFfmpeg} 
                disabled={downloadingFfmpeg}
                className="download-ffmpeg-btn"
              >
                {downloadingFfmpeg ? 'Downloading...' : 'Download FFmpeg'}
              </button>
            </div>
          )}
          <div className="output-selector">
            <label>Output Directory:</label>
            <div className="directory-picker">
              <input
                type="text"
                value={outputPath}
                readOnly
                className="output-input"
                placeholder="Select output directory..."
              />
              <button onClick={selectOutputDirectory} className="browse-btn">
                📁 Browse
              </button>
              <button onClick={openOutputDirectory} className="open-btn">
                📂 Open
              </button>
            </div>
          </div>
          <button
            onClick={compressFiles}
            disabled={isProcessing || files.every(f => f.status === 'completed')}
            className="compress-btn"
          >
            {isProcessing ? 'Compressing...' : 'Compress All'}
          </button>
        </div>
      )}
    </main>
  );
}

export default App;