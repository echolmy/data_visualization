# Scientific Data Visualization Tool

A high-performance scientific data visualization application built with Rust and the Bevy engine, specifically designed for 3D scientific data visualization with specialized support for VTK (Visualization Toolkit) format files.

## Key Features

### File Format Support
- **VTK File Formats**: Complete support for VTK Legacy and XML format file import and parsing
  - Legacy: `.vtk`, `.vtu` - Support for unstructured grids and polygon data
  - XML: `.vtp`, `.vts`, `.vtr`, `.vti` - In development
- **Multiple Data Types**:
  - Unstructured Grid
  - PolyData

### Mesh Processing Capabilities
- **Triangulation**: Automatic conversion of complex polygons and polyhedral cells to triangles
- **Mesh Subdivision**: Support for triangle mesh subdivision with higher mesh density
  - Note: Cannot be used with LOD system simultaneously
- **LOD System**: Automatic generation of multiple Level of Detail (LOD) levels
  - Support for distance-based automatic LOD switching

### Visualization Rendering
- **3D Mesh Rendering**: Support for triangulated mesh and various cell topology visualization
- **Wireframe Mode**: Toggle between solid and wireframe rendering modes
- **Color Scalar Mapping**: Support for mesh coloring based on scalar data
- **Multiple Color Maps**:
  - Default rainbow color mapping
  - Heat map mapping
  - Viridis mapping
  - High-resolution rainbow mapping
- **Real-time Color Updates**: Support for real-time mesh color mapping updates

### Time Series Animation System
- **Time Series Import**: Support for importing multiple time-step VTK files as animation sequences
- **Two-Stage Loading**:
  - Stage 1: Import frame 0 as static model
  - Stage 2: Load scalar data for all time steps
- **Real-time Animation Playback**: Support for play/pause and time-step control
- **Scalar Data Animation**: Support for time-series based scalar attribute animation
  - Note: Does not support mesh subdivision operations

### Dynamic Effects
- **CPU Wave Generation**: Generate mathematical wave surfaces with real-time parameter adjustment
- **GPU Shader Waves**: High-performance dynamic wave effects using GPU shaders
- **Real-time Animation**: Time-driven wave animation effects

### Interactive Camera System
- **Free-Flight Camera**: Complete 3D camera control system
- **Mouse Controls**:
  - Right-click drag: Rotate view
  - Mouse wheel: Zoom
- **Camera Movement**:
  - W/A/S/D: Forward/backward/left/right movement
  - Q/E: Up/down movement
  - Arrow keys: Alternative movement controls
  - R: Reset camera to default position
  - Shift + W/A/S/D/Q/E: Fast movement (10x speed)
- **Model Transformation**:
  - Alt + Left mouse drag: Rotate model** (Windows/Linux) / Option + Left mouse drag (macOS)
  - Alt + Middle mouse drag: Translate model position** (Windows/Linux) / Option + Middle mouse drag (macOS)

### User Interface
- **Modern GUI**: Intuitive user interface based on egui
- **Menu System**:
  - File menu: File import, time series import, and exit
  - View menu: Rendering mode toggle, clear meshes
  - Mesh menu: Mesh subdivision, wave generation
- **Time Series Control Panel**:
  - Play/pause controls
  - Time step slider
  - Current file display
  - Loading status indicator
- **Color Bar Configuration**: Real-time color mapping parameter adjustment

## Main Dependencies

- `bevy` (0.15.0) - Modern game engine framework
- `bevy_egui` (0.31.1) - GUI integration library
- `vtkio` (0.7.0-rc1) - VTK file format support
- `rfd` (0.15.0) - Cross-platform file dialog
- `bevy_obj` (0.15.0) - OBJ file format support
- `bevy_atmosphere` (0.12.2) - Atmospheric effects rendering

## Quick Start

### Prerequisites
This application requires a Rust development environment. If you don't have Rust installed:

1. Install Rust from https://www.rust-lang.org/tools/install
2. Verify installation: `rustc --version`

### Running from Source
1. Clone the repository:
```bash
git clone https://github.com/echolmy/data_visualization.git
cd data_visualization
```

2. Build and run:
```bash
cargo run
```

## Test Data

### Sample VTK Files
The repository includes several sample VTK data files in the `assets/` directory for testing:
- `bunny.vtk` - 3D bunny model
- `sphere_order1.vtu` and `sphere_order2.vtu` - Sphere models with different orders
- `torus.vtu` - Torus model

### Time Series Data
Due to large file sizes, time series data is not hosted directly in the Git repository. You can download the time series sample data from:

- **Google Drive**: https://drive.google.com/drive/folders/1qAVuYsR6JV8zO8XOnjt7I4SLBL4fZ-Yw?usp=drive_link
- **Dropbox**: https://www.dropbox.com/scl/fi/n4tumgwakxyoqt1isz50x/sequence.zip?rlkey=8184dnduq94w6fwq9phdm245e&st=3khfdvk0&dl=0

After downloading, extract the files to the `assets/sequence/` directory to test time series animation features.

## Usage Guide

### Basic Operations
1. **Import Single File**: Import VTK or OBJ files through menu `File > Import`
2. **Import Time Series**: Select folder containing multiple time-step files through `File > Import Time Series`
3. Toggle rendering mode through `View > Wireframe`

### Time Series Animation
1. Use `File > Import Time Series` to import time series folder
2. Use time series control panel:
   - Click play/pause button to control animation
   - Drag time-step slider to jump to specific time
   - View current loaded file information

### Advanced Features
1. **Mesh Subdivision**: Subdivide loaded meshes in `Mesh > Subdivide`
2. **Wave Generation**:
   - `Mesh > Create Wave Surface (CPU)`: Generate CPU-computed wave surface
   - `Mesh > Create Wave Surface (GPU Shader)`: Generate GPU shader-driven wave surface

## Project Structure

```
src/
├── main.rs              # Application entry point
├── animation.rs         # Time series animation system
├── mesh/                # Mesh processing modules
│   ├── vtk.rs          # VTK file parsing and geometry data extraction
│   ├── subdivision.rs   # Mesh subdivision algorithms
│   ├── triangulation.rs # Triangulation algorithms
│   ├── color_maps.rs   # Color mapping tables
│   └── wave.rs         # Wave surface generation
├── ui/                  # User interface modules
│   └── events.rs       # UI event system
├── camera.rs            # Camera control system
├── lod.rs              # Level of Detail (LOD) system
├── model_transform.rs   # Model transformation functionality
├── render/              # Rendering functionality
│   └── wave_material.rs # GPU wave shader material
└── environment.rs       # Environment and lighting setup
```

## Technical Features

- **High-Performance Rendering**: Modern rendering pipeline based on Bevy engine
- **Memory Safety**: Memory safety guarantees from Rust language
- **Modular Design**: Clear module structure, easy to extend
- **Cross-Platform**: Support for Windows, macOS, and Linux
- **GPU Acceleration**: Support for GPU shader-implemented dynamic effects
- **Event-Driven Architecture**: Use event system for inter-module communication

## Development Status

The project is currently in active development.

## Building Release Version

For optimal performance, build in release mode:

```bash
cargo build --release
```
