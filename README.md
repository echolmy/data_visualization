# Data Visualization

A scientific data visualization tool built with Rust and Bevy engine, specifically designed for visualizing complex VTK (Visualization Toolkit) data formats commonly used in scientific simulations and data visualization.

## Project Background & Motivation

This project addresses the limitations of traditional VTK-based visualization tools like Paraview by leveraging modern game engine technology. While existing tools provide basic functionality, they often lack the performance and interactivity offered by modern game engines.

Key advantages of our approach:

1. **High Performance and Safety**
   - Utilizes Rust's memory safety and high-performance capabilities
   - Designed to handle large-scale scientific data efficiently
   - Prevents program crashes through Rust's safety guarantees

2. **Modularity and Extensibility**
   - Built on Bevy's Entity-Component-System (ECS) architecture
   - Supports modular development and flexible extensions
   - Enables advanced features like multi-level detail (LOD) rendering

3. **Interactivity and Real-Time Performance**
   - Powered by Bevy's rendering capabilities
   - Real-time data interaction and visualization
   - Intuitive user interface for data analysis

## Current Features

- VTK file import and parsing support
- Geometric data visualization with triangulation
- Cell topology rendering
- Color scalar data processing for specific cell topologies
- Basic interaction controls and user interface
- CPU-based data processing

## Planned Features

### VTK Data Integration
- Support for scalar and vectorial data
- Multiple element type support

### Mesh Rendering
- High-resolution surface and mesh handling (millions of triangles)
- Multi-level detail (LOD) techniques for dynamic resolution
- GPU-accelerated mesh processing
- Custom material system for advanced visual effects

### Enhanced Interaction
- Advanced data exploration tools
- Real-time rotation, zooming, and cross-sectional views
- Refined UI based on professional visualization tools (Gmsh, ParaView)

### Solver Integration
- Integration with PDE solvers (e.g., bempp package)
- Solver output pipeline for rendering
- VTK file export functionality
- Post-processing and analysis tools
- Error evaluation capabilities

## Prerequisites

- Rust (2021 edition)
- Cargo (Rust's package manager)

## Dependencies

- bevy (0.15.0) - Game engine
- bevy_egui (0.31.1) - GUI integration
- bevy_obj (0.15.0) - 3D model loading
- vtkio (0.7.0-rc1) - VTK file format support
- bevy_atmosphere (0.12.2) - Atmospheric effects
- rfd (0.15.0) - File dialogs

## Installation

1. Clone the repository:
```bash
git clone [repository-url]
cd data_visualization
```

2. Build the project:
```bash
cargo build
```

3. Run the application:
```bash
cargo run
```

## Usage

1. Launch the application using `cargo run`
2. Use the GUI interface to:
   - Load VTK files
   - Adjust visualization parameters
   - Control camera views
   - Process and analyze scientific data

### Camera Controls
- Left mouse button: Rotate camera
- Right mouse button: Pan camera
- Mouse wheel: Zoom in/out

## Development

The project structure:
- `src/main.rs` - Application entry point and core setup
- `src/mesh/` - Mesh handling and VTK file processing
- `src/camera/` - Camera control system
- `src/ui/` - User interface components

## Building for Release

For optimal performance, build in release mode:

```bash
cargo build --release
```

The optimized executable will be available in `target/release/`.
