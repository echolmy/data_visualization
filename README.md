# Scientific Data Visualization Tool

A high-performance scientific data visualization application built with Rust and the Bevy engine, specifically designed for 3D scientific data visualization with specialized support for VTK (Visualization Toolkit) format files.

## Key Features

### File Format Support
- **VTK File Formats**: Complete support for VTK Legacy and XML format file import and parsing
  - Legacy: `.vtk`, `.vtu` - Support for unstructured grids and polygon data
  - XML: `.vtp`, `.vts`, `.vtr`, `.vti` - In development, partial format support
- **OBJ Format**: Support for standard OBJ 3D model files
- **Multiple Data Types**:
  - Unstructured Grid
  - PolyData (Polygon Data)

### Time Series Animation System
- **Time Series Import**: Support for importing multiple time-step VTK files as animation sequences
- **Automatic Sorting**: Intelligent recognition and automatic sorting of time-step numbers in filenames
- **Two-Stage Loading**:
  - Stage 1: Import frame 0 as static model
  - Stage 2: Load scalar data for all time steps
- **Real-time Animation Playback**: Support for play/pause and time-step control
- **Scalar Data Animation**: Support for time-series based scalar attribute animation

### Mesh Processing Capabilities
- **Triangulation**: Automatic conversion of complex polygons and polyhedral cells to triangles
- **Mesh Subdivision**: Support for triangle mesh subdivision with higher mesh density
- **LOD System**: Automatic generation of multiple Level of Detail (LOD) levels
  - Support for distance-based automatic LOD switching
  - Maintains color mapping consistency across different LOD levels

### Visualization Rendering
- **3D Mesh Rendering**: Support for triangulated mesh and various cell topology visualization
- **Wireframe Mode**: Toggle between solid and wireframe rendering modes (Hotkey: Z)
- **Color Scalar Mapping**: Support for mesh coloring based on scalar data
- **Multiple Color Maps**:
  - Default rainbow color mapping
  - Heat map mapping
  - Viridis mapping
  - High-resolution rainbow mapping
- **Automatic Color Range**: Automatic adjustment of color mapping range based on data
- **Real-time Color Updates**: Support for real-time mesh color mapping updates

### Dynamic Effects
- **CPU Wave Generation**: Generate mathematical wave surfaces with real-time parameter adjustment
- **GPU Shader Waves**: High-performance dynamic wave effects using GPU shaders
- **Real-time Animation**: Time-driven wave animation effects

### Interactive Camera System
- **Free-Flight Camera**: Complete 3D camera control system
- **Mouse Controls**:
  - Right-click drag: Rotate view
  - Mouse wheel: Zoom
  - Left-click drag: Pan view
- **Camera Movement**:
  - W/A/S/D: Forward/backward/left/right movement
  - Q/E: Up/down movement
  - Arrow keys: Alternative movement controls
  - R: Reset camera to default position
  - **Shift + W/A/S/D/Q/E: Fast movement (10x speed)**
- **Model Transformation**:
  - **Alt + Left mouse drag: Rotate model**
  - **Alt + Middle mouse drag: Translate model position**
  - **Alt + R: Reset model transformation**
- **Smart Focus**: Automatically adjust camera position to fit loaded models

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

1. Clone the repository:
```bash
git clone [repository-url]
cd data_visualization
```

2. Build and run:
```bash
cargo run
```

## Usage Guide

### Basic Operations
1. **Import Single File**: Import VTK or OBJ files through menu `File > Import`
2. **Import Time Series**: Select folder containing multiple time-step files through `File > Import Time Series`
3. Toggle rendering mode through `View > Wireframe`

### Time Series Animation
1. Use `File > Import Time Series` to import time series folder
2. System automatically recognizes and sorts time-step files
3. Use time series control panel:
   - Click play/pause button to control animation
   - Drag time-step slider to jump to specific time
   - View current loaded file information

### Advanced Features
1. **Mesh Subdivision**: Subdivide loaded meshes in `Mesh > Subdivide`
2. **Wave Generation**:
   - `Mesh > Create Wave Surface (CPU)`: Generate CPU-computed wave surface
   - `Mesh > Create Wave Surface (GPU Shader)`: Generate GPU shader-driven wave surface
3. **Color Mapping Adjustment**: Real-time adjustment of color mapping parameters through color bar panel

### Intuitive Control System
This tool provides intuitive dual control methods:

#### Camera Control (Change Viewing Angle)
- **Operation**: W/A/S/D movement, right mouse button rotation, wheel zoom
- **Effect**: You "fly" through 3D space to view the model

#### Model Transformation (Adjust Model Itself)
- **Operation**: Alt + Left mouse drag (rotation), Alt + Middle mouse drag (translation)
- **Effect**: Adjust model position and orientation for observation from various angles

**Recommended Usage**: Combine both methods for optimal viewing experience

### Supported File Formats
- **VTK Files**: `.vtk`, `.vtu` - Support for scalar attributes, color attributes, and vector attributes
- **OBJ Files**: `.obj` - Standard 3D model files
- **Time Series**: Support for multi-time-step VTK files with automatic filename sorting

### Large Model Optimization
For large scientific data models, the system provides optimized navigation experience:
- **Smart Zoom**: Wheel zoom speed automatically adjusts based on distance - faster zoom when farther away
- **Fast Movement**: Hold Shift + WASD for 10x speed fast movement
- **Auto Focus**: Automatically adjust camera to suitable position after model loading
- **Distance Protection**: Prevent operation difficulties caused by camera being too near or far
- **LOD System**: Automatically simplify distant meshes to improve performance

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
- **Smart Memory Management**: Automatic optimization of memory usage for large datasets

## Development Status

The project is currently in active development with core functionality implemented:

### Completed Features
- Complete VTK Legacy format support
- Time series animation system
- LOD system
- Basic rendering and interaction functionality
- Mesh processing and subdivision
- Color mapping system

### In Development
- Complete VTK XML format support (.vtp, .vts, .vtr, .vti)
- More color mapping options
- Advanced rendering effects

### Planned Features
- Volume rendering support
- More file format support
- Data analysis tool integration

## Building Release Version

For optimal performance, build in release mode:

```bash
cargo build --release
```

The release version provides:
- Faster rendering performance
- Lower memory usage
- Smoother animation playback
- Better large model processing capability
