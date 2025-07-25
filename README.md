# 数据可视化工具

基于 Rust 和 Bevy 引擎构建的科学数据可视化工具，专门用于三维科学数据的可视化，特别是对 VTK（Visualization Toolkit）格式文件的处理和展示。

## 核心特性

### 文件格式支持
- **VTK 文件格式**：完整支持 VTK Legacy 和 XML 格式文件的导入和解析
- **OBJ 格式**：支持标准 OBJ 3D 模型文件
- **多种数据类型**：
  - 非结构化网格（Unstructured Grid）
  - 多边形数据（PolyData）

### 网格处理功能
- **智能三角化**：自动将复杂多边形和多面体单元转换为三角形
- **网格细分**：支持三角网格细分，提供更高的网格密度

### 可视化渲染
- **3D 网格渲染**：支持三角化网格和各种单元拓扑的可视化
- **线框模式**：可在实体和线框渲染模式之间切换（快捷键：Z）
- **颜色标量映射**：支持基于标量数据的网格着色
- **多种颜色映射表**：
  - 默认彩虹色映射
  - 热力图映射
  - Viridis 映射
  - 高分辨率彩虹映射

### 动态效果
- **CPU 波浪生成**：生成数学波浪表面，支持实时参数调整
- **GPU 着色器波浪**：使用 GPU 着色器实现高性能动态波浪效果
- **实时动画**：时间驱动的波浪动画效果

### 交互式相机系统
- **自由飞行相机**：完整的 3D 相机控制系统
- **鼠标控制**：
  - 右键拖拽：旋转视角
  - 鼠标滚轮：缩放
  - 左键拖拽：平移视图
- **相机控制**：
  - W/A/S/D：前后左右移动
  - Q/E：上下移动  
  - 方向键：备用移动控制
  - R：重置相机到默认位置
  - **Shift + W/A/S/D/Q/E：快速移动（10倍速度）**
- **模型变换**：
  - **Alt + 鼠标左键拖拽：旋转模型**
  - **Alt + 鼠标中键拖拽：平移模型位置**
  - **Alt + R：重置模型变换**
- **智能对焦**：自动调整相机位置以适应加载的模型

### 用户界面
- **现代 GUI**：基于 egui 的直观用户界面
- **菜单系统**：
  - File 菜单：文件导入和退出
  - View 菜单：渲染模式切换、清除网格
  - Mesh 菜单：网格细分、波浪生成
- **键盘快捷键**：
  - Delete：清除所有用户导入的网格
  - Z：切换线框模式
  - R：重置相机位置
  - Shift：配合WASD进行快速移动
  - **Ctrl：进入模型变换模式**
    - Ctrl + X/C/Y/H/Z/V：旋转模型
    - Ctrl + +/-：缩放模型
    - Ctrl + 方向键：平移模型
    - Ctrl + 0：重置模型变换

## 主要依赖

- `bevy` (0.15.0) - 现代游戏引擎框架
- `bevy_egui` (0.31.1) - GUI 集成库
- `vtkio` (0.7.0-rc1) - VTK 文件格式支持
- `rfd` (0.15.0) - 跨平台文件对话框
- `bevy_obj` (0.15.0) - OBJ 文件格式支持
- `bevy_atmosphere` (0.12.2) - 大气效果渲染

## 快速开始

1. 克隆项目：
```bash
git clone [repository-url]
cd data_visualization
```

2. 构建并运行：
```bash
cargo run
```

## 使用方法

### 基本操作
1. 启动应用程序后，通过菜单栏 `File > Import` 导入 VTK 或 OBJ 文件
2. 使用鼠标控制相机：
   - 右键拖拽：旋转视角
   - 鼠标滚轮：缩放视图
   - 左键拖拽：平移视图
3. 通过 `View > Wireframe` 或按 Z 键切换渲染模式

### 高级功能
1. **网格细分**：在 `Mesh > Subdivide` 中对已加载的网格进行细分
2. **波浪生成**：
   - `Mesh > Create Wave Surface (CPU)`：生成 CPU 计算的波浪表面
   - `Mesh > Create Wave Surface (GPU Shader)`：生成 GPU 着色器驱动的波浪表面
3. **清除网格**：使用 `View > Clear User Meshes` 或按 Delete 键清除所有导入的网格

### 简洁的控制系统
本工具提供了直观的双重控制方式：

#### 🎥 相机控制（改变观察角度）
- **操作**：W/A/S/D移动，鼠标右键旋转，滚轮缩放
- **效果**：你在3D空间中"飞行"查看模型

#### 🎭 模型变换（调整模型本身）
- **操作**：Alt + 鼠标左键拖拽（旋转）、Alt + 鼠标中键拖拽（平移）
- **效果**：调整模型位置和朝向，方便从各个角度观察

**推荐使用**：两种方式结合使用，获得最佳观察体验

### 支持的文件格式
- **VTK 文件**：`.vtk`, `.vtu` - 支持标量属性、颜色属性和向量属性
- **OBJ 文件**：`.obj` - 标准 3D 模型文件

### 大模型优化
对于大型科学数据模型，系统提供了优化的导航体验：
- **智能缩放**：滚轮缩放速度根据距离自动调整，距离越远缩放越快
- **快速移动**：按住 Shift + WASD 进行10倍速快速移动
- **自动对焦**：模型加载后自动调整相机到合适位置
- **距离保护**：防止相机过近或过远造成的操作困难

## 项目结构

```
src/
├── main.rs              # 应用程序入口点
├── mesh/                # 网格处理模块
│   ├── vtk.rs          # VTK 文件解析和几何数据提取
│   ├── subdivision.rs   # 网格细分算法
│   ├── triangulation.rs # 三角化算法
│   ├── color_maps.rs   # 颜色映射表
│   └── wave.rs         # 波浪表面生成
├── ui/                  # 用户界面模块
│   └── events.rs       # UI 事件系统
├── camera/              # 相机控制系统
├── render/              # 渲染功能
│   └── wave_material.rs # GPU 波浪着色器材质
└── environment.rs       # 环境和光照设置
```

## 技术特点

- **高性能渲染**：基于 Bevy 引擎的现代渲染管线
- **内存安全**：Rust 语言的内存安全保证
- **模块化设计**：清晰的模块结构，易于扩展
- **跨平台**：支持 Windows、macOS 和 Linux
- **GPU 加速**：支持 GPU 着色器实现的动态效果

## 开发状态

项目目前处于活跃开发阶段，核心功能已经实现。未来将继续添加更多高级渲染功能和数据格式支持。

## 构建发布版本

为获得最佳性能，请使用发布模式构建：

```bash
cargo build --release
```
