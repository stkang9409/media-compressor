# Media Compressor

A powerful cross-platform desktop application for compressing images and videos with ease. Built with Tauri and React for optimal performance and small binary size.

## ✨ Features

- 🖼️ **Image Compression**: Support for JPEG, PNG, WebP formats
- 🎥 **Video Compression**: Compress MP4, MOV, AVI, and more
- 🚀 **Fast Processing**: Native performance with Rust backend
- 💾 **Batch Processing**: Compress multiple files at once
- 📊 **Real-time Preview**: See compression results before saving
- 🎯 **Custom Quality**: Adjust compression levels to your needs
- 🌍 **Cross-Platform**: Works on Windows, macOS, and Linux

## 📥 Download

### [⬇️ Download Latest Release](https://github.com/stkang9409/media-compressor/releases/latest)

Visit the releases page to download the appropriate version for your operating system. The release page includes detailed instructions for each platform.

## 🚀 Quick Start

### For Users

1. Download the appropriate version for your operating system from the links above
2. Install the application:
   - **Windows**: Run the `.exe` installer
   - **macOS**: Open the `.dmg` file and drag the app to Applications
   - **Linux**:
     - AppImage: Make it executable with `chmod +x` and run
     - deb: Install with `sudo dpkg -i media-compressor_*.deb`

### For Developers

#### Prerequisites

- [Node.js](https://nodejs.org/) (v18 or later)
- [Rust](https://www.rust-lang.org/) (latest stable)
- [Tauri CLI](https://tauri.app/v1/guides/getting-started/prerequisites)

#### Installation

```bash
# Clone the repository
git clone https://github.com/stkang9409/media-compressor.git
cd media-compressor

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## 📋 Usage

1. **Launch the application**
2. **Select files**: Drag and drop or click to browse
3. **Choose compression settings**:
   - Quality level (1-100)
   - Output format
   - Resize options (optional)
4. **Preview results**: Check file size reduction
5. **Save**: Choose output location and save compressed files

## 🛠️ Tech Stack

- **Frontend**: React + TypeScript + Vite
- **Backend**: Rust + Tauri
- **Image Processing**: Native image libraries
- **Video Processing**: FFmpeg bindings

## 📝 Configuration

The app stores settings in:

- **Windows**: `%APPDATA%/media-compressor`
- **macOS**: `~/Library/Application Support/media-compressor`
- **Linux**: `~/.config/media-compressor`

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Tauri](https://tauri.app/)
- UI powered by [React](https://reactjs.org/)
- Icons and design resources from the open-source community

## 📞 Support

If you encounter any issues or have questions, please [open an issue](https://github.com/stkang9409/media-compressor/issues) on GitHub.

---

Made with ❤️ by the Media Compressor Team
