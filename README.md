# Data Visualization

A scientific data visualization tool built with Rust and Bevy engine, specifically designed for visualizing 3D scientific data in VTK (Visualization Toolkit) format.

## Key Features

- **VTK File Support**: Import and parse VTK format files
- **3D Mesh Rendering**: Visualization of triangulated meshes and various cell topologies
- **Colored Scalar Data**: Color mesh cells based on scalar data
- **Wireframe Mode**: Toggle between solid/wireframe rendering modes
- **Higher-Order Mesh Conversion**: Convert first-order meshes to higher-order meshes (2nd order)
- **Interactive Camera**: Support for rotation, panning, zooming and other basic interactions
- **Modern GUI**: Intuitive user interface based on egui

## Main Dependencies

- `bevy` (0.15.0) - Game engine framework
- `bevy_egui` (0.31.1) - GUI integration
- `vtkio` (0.7.0-rc1) - VTK file format support
- `rfd` (0.15.0) - File dialogs

## Quick Start

1. Clone the project:
```bash
git clone [repository-url]
cd data_visualization
```

2. Build and run:
```bash
cargo run
```

## Usage

1. After launching the application, import VTK files through menu bar `File > Import`
2. Control camera with mouse:
   - Left drag: Rotate view
   - Right drag: Pan view
   - Mouse wheel: Zoom
3. Toggle rendering mode through `View > Wireframe`
4. Convert mesh to higher-order through `Mesh > Convert to Second Order` (if supported)

## Project Structure

```
src/
├── main.rs           # Application entry point
├── mesh.rs          # Mesh processing and VTK file parsing
├── ui.rs            # User interface components
├── camera.rs        # Camera control system
├── render.rs        # Rendering related functionality
└── environment.rs   # Environment and lighting setup
```

## Development Status

The project is currently in development stage. Core functionality has been implemented. Advanced rendering features and support for more data formats will be added in the future.

## Building Release Version

For optimal performance, build in release mode:

```bash
cargo build --release
```
