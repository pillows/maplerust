use eframe::egui;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use wz_reader::property::get_image;
use wz_reader::util::walk_node;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzFile, WzImage, WzNode, WzNodeArc, WzNodeCast, WzNodeName, WzObjectType, WzReader};

#[derive(Default)]
struct WzViewerApp {
    // File management
    current_file: Option<PathBuf>,
    root_node: Option<WzNodeArc>,
    error_message: Option<String>,
    
    // UI state
    selected_path: Option<String>,
    expanded_paths: std::collections::HashSet<String>,
    
    // Image viewing
    current_image: Option<egui::TextureHandle>,
    image_path: Option<String>,
}

impl eframe::App for WzViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top panel for menu and header
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.heading("WZ/IMG File Viewer");
            
            // Top menu bar
            ui.horizontal(|ui| {
                if ui.button("Open File").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("IMG files", &["img"])
                        .add_filter("WZ files", &["wz"])
                        .add_filter("All files", &["*"])
                        .pick_file()
                    {
                        self.load_file(&path);
                    }
                }
                
                if ui.button("Export Structure").clicked() {
                    if let Some(root) = &self.root_node {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Text files", &["txt"])
                            .save_file()
                        {
                            self.export_structure(root, &path);
                        }
                    }
                }
                
                if let Some(file) = &self.current_file {
                    ui.label(format!("File: {}", file.display()));
                }
            });
            
            ui.separator();
            
            // Error display
            if let Some(error) = &self.error_message {
                ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
            }
        });
        
        // Left side panel for tree view
        egui::SidePanel::left("tree_panel")
            .resizable(true)
            .default_width(400.0)
            .min_width(300.0)
            .max_width(600.0)
            .show(ctx, |ui| {
                ui.heading("Structure");
                egui::ScrollArea::vertical()
                    .id_source("tree_scroll")
                    .show(ui, |ui| {
                        if let Some(root) = self.root_node.as_ref() {
                            // Clone the Arc to avoid borrow checker issues
                            let root_clone = Arc::clone(root);
                            // Start with empty path for root node
                            self.render_tree(ui, &root_clone, "");
                        } else {
                            ui.label("No file loaded. Click 'Open File' to load an IMG file.");
                        }
                    });
            });

        // Central panel for details/image view
        egui::CentralPanel::default().show(ctx, |ui| {
                egui::ScrollArea::both()
                    .id_source("details_scroll")
                    .show(ui, |ui| {
                        ui.heading("Details");
                        
                        if let Some(path) = &self.selected_path {
                            // Get the actual full path for display
                            let display_path = if path.is_empty() {
                                if let Some(root) = &self.root_node {
                                    root.read().unwrap().get_full_path()
                                } else {
                                    String::new()
                                }
                            } else {
                                // Reconstruct full path by prepending root name
                                if let Some(root) = &self.root_node {
                                    let root_name = root.read().unwrap().name.to_string();
                                    format!("{}/{}", root_name, path)
                                } else {
                                    path.clone()
                                }
                            };
                            ui.label(format!("Path: {}", display_path));
                            
                            // Try to load and display PNG if it's a PNG node
                            if let Some(root) = &self.root_node {
                                // If path is empty, use root directly; otherwise use at_path_parsed
                                // which automatically parses nodes along the path
                                let node_opt = if path.is_empty() {
                                    Some(Arc::clone(root))
                                } else {
                                    // Ensure root is parsed first
                                    let root_parsed = {
                                        let mut root_write = root.write().unwrap();
                                        root_write.parse(root).is_ok()
                                    };
                                    
                                    if !root_parsed {
                                        None
                                    } else {
                                        // Try at_path_parsed first (parses nodes along the path)
                                        let root_read = root.read().unwrap();
                                        match root_read.at_path_parsed(path) {
                                            Ok(node) => Some(node),
                                            Err(_) => {
                                                // Fallback to at_path if at_path_parsed fails
                                                // This might happen if some nodes can't be parsed
                                                root_read.at_path(path)
                                            }
                                        }
                                    }
                                };
                                
                                if let Some(node) = node_opt {
                                    // Ensure the node itself is parsed if it's a Property node
                                    // This is needed to access PNG data and other properties
                                    {
                                        let parse_result = {
                                            let mut node_write = node.write().unwrap();
                                            node_write.parse(&node)
                                        };
                                        
                                        if let Err(e) = parse_result {
                                            ui.colored_label(
                                                egui::Color32::YELLOW,
                                                format!("Warning: Failed to parse node: {:?}", e),
                                            );
                                        }
                                    }
                                    
                                    let node_read = node.read().unwrap();
                                        
                                        let type_str = match &node_read.object_type {
                                            WzObjectType::Directory(_) => "Directory",
                                            WzObjectType::Image(_) => "Image",
                                            WzObjectType::File(_) => "File",
                                            WzObjectType::MsFile(_) => "MsFile",
                                            WzObjectType::MsImage(_) => "MsImage",
                                            WzObjectType::Property(p) => match p {
                                                wz_reader::property::WzSubProperty::Convex => "Convex",
                                                wz_reader::property::WzSubProperty::Sound(_) => "Sound",
                                                wz_reader::property::WzSubProperty::PNG(_) => "PNG",
                                                wz_reader::property::WzSubProperty::Property => "Property",
                                            },
                                            WzObjectType::Value(v) => match v {
                                                wz_reader::property::WzValue::Null => "Null",
                                                wz_reader::property::WzValue::Short(_) => "Short",
                                                wz_reader::property::WzValue::Int(_) => "Int",
                                                wz_reader::property::WzValue::Long(_) => "Long",
                                                wz_reader::property::WzValue::Float(_) => "Float",
                                                wz_reader::property::WzValue::Double(_) => "Double",
                                                wz_reader::property::WzValue::String(_) => "String",
                                                wz_reader::property::WzValue::ParsedString(_) => "ParsedString",
                                                wz_reader::property::WzValue::Vector(_) => "Vector",
                                                wz_reader::property::WzValue::UOL(_) => "UOL",
                                                wz_reader::property::WzValue::RawData(_) => "RawData",
                                                wz_reader::property::WzValue::Video(_) => "Video",
                                                wz_reader::property::WzValue::Lua(_) => "Lua",
                                            },
                                        };
                                        
                                        ui.label(format!("Type: {}", type_str));
                                        
                                        // Check if it's a PNG using try_as_png (more reliable)
                                        if node_read.try_as_png().is_some() {
                                            ui.separator();
                                            ui.heading("PNG Image");
                                            
                                            // Get PNG info
                                            if let WzObjectType::Property(
                                                wz_reader::property::WzSubProperty::PNG(png_data)
                                            ) = &node_read.object_type {
                                                ui.label(format!("Size: {}x{}", png_data.width, png_data.height));
                                            }
                                            
                                            // Only reload if path changed
                                            if self.image_path.as_ref() != Some(path) {
                                                // Use the get_image helper which handles _inlink/_outlink
                                                match get_image(&node) {
                                                    Ok(dynamic_img) => {
                                                        let rgba = dynamic_img.to_rgba8();
                                                        let size = [rgba.width() as usize, rgba.height() as usize];
                                                        let pixels = rgba.as_raw();
                                                        
                                                        // Convert to egui image
                                                        let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                                            size,
                                                            pixels,
                                                        );
                                                        
                                                        // Create or update texture with unique ID
                                                        let texture_id = format!("png_{}", path.replace("/", "_"));
                                                        let texture = ctx.load_texture(
                                                            &texture_id,
                                                            color_image,
                                                            egui::TextureOptions::LINEAR,
                                                        );
                                                        
                                                        self.current_image = Some(texture);
                                                        self.image_path = Some(path.clone());
                                                    }
                                                    Err(e) => {
                                                        ui.colored_label(
                                                            egui::Color32::RED,
                                                            format!("Failed to extract PNG: {:?}", e),
                                                        );
                                                        self.current_image = None;
                                                        self.image_path = None;
                                                    }
                                                }
                                            }
                                            
                                            // Display image if loaded
                                            if let Some(tex) = &self.current_image {
                                                ui.separator();

                                                // Get size from PNG data if available
                                                let (width, height) = if let WzObjectType::Property(
                                                    wz_reader::property::WzSubProperty::PNG(png_data)
                                                ) = &node_read.object_type {
                                                    (png_data.width as usize, png_data.height as usize)
                                                } else {
                                                    (100, 100) // fallback
                                                };

                                                // Use fixed maximum dimensions for image display
                                                // This prevents the image from expanding beyond reasonable bounds
                                                const MAX_IMAGE_WIDTH: f32 = 600.0;
                                                const MAX_IMAGE_HEIGHT: f32 = 500.0;

                                                // Calculate scale to fit within max dimensions
                                                let scale = (MAX_IMAGE_WIDTH / width as f32)
                                                    .min(MAX_IMAGE_HEIGHT / height as f32)
                                                    .min(1.0); // Don't scale up, only down

                                                let display_size = egui::vec2(
                                                    width as f32 * scale,
                                                    height as f32 * scale,
                                                );

                                                ui.label(format!("Original: {}x{}", width, height));
                                                ui.label(format!("Display: {:.0}x{:.0} (scale: {:.2})",
                                                    display_size.x, display_size.y, scale));

                                                ui.add_space(5.0);
                                                ui.image((tex.id(), display_size));
                                            }
                                        } else {
                                            // Show other node information
                                            if !node_read.children.is_empty() {
                                                ui.separator();
                                                ui.heading(format!("Children ({})", node_read.children.len()));
                                                for (name, _) in &node_read.children {
                                                    ui.label(name.to_string());
                                                }
                                            }
                                            
                                            // Show value if it's a Value node
                                            match &node_read.object_type {
                                                WzObjectType::Value(v) => {
                                                    ui.separator();
                                                    ui.heading("Value");
                                                    match v {
                                                        wz_reader::property::WzValue::Short(val) => {
                                                            ui.label(format!("Short: {}", val));
                                                        }
                                                        wz_reader::property::WzValue::Int(val) => {
                                                            ui.label(format!("Int: {}", val));
                                                        }
                                                        wz_reader::property::WzValue::Long(val) => {
                                                            ui.label(format!("Long: {}", val));
                                                        }
                                                        wz_reader::property::WzValue::Float(val) => {
                                                            ui.label(format!("Float: {}", val));
                                                        }
                                                        wz_reader::property::WzValue::Double(val) => {
                                                            ui.label(format!("Double: {}", val));
                                                        }
                                                        wz_reader::property::WzValue::String(val) => {
                                                            match val.get_string() {
                                                                Ok(s) => {
                                                                    ui.label(format!("String: {}", s));
                                                                }
                                                                Err(e) => {
                                                                    ui.label(format!("String: [Error: {:?}]", e));
                                                                }
                                                            }
                                                        }
                                                        wz_reader::property::WzValue::ParsedString(val) => {
                                                            ui.label(format!("ParsedString: {}", val));
                                                        }
                                                        wz_reader::property::WzValue::Vector(vec) => {
                                                            ui.label(format!("Vector: ({}, {})", vec.0, vec.1));
                                                        }
                                                        _ => {
                                                            ui.label(format!("{:?}", v));
                                                        }
                                                    }
                                                }
                                                WzObjectType::Property(wz_reader::property::WzSubProperty::Property) => {
                                                    ui.separator();
                                                    ui.label("Property node - expand to see children");
                                                }
                                                _ => {
                                                    // Other types - no additional info to show
                                                }
                                            }
                                        }
                                } else {
                                    ui.colored_label(
                                        egui::Color32::RED,
                                        format!("Node not found at path: {}", path),
                                    );
                                    ui.label("This may happen if:");
                                    ui.label("• The node hasn't been parsed yet");
                                    ui.label("• The path is incorrect");
                                    ui.label("• The file structure changed");
                                }
                            }
                        } else {
                            ui.label("Select a node from the tree to view details.");
                        }
                    });
        });
    }
}

impl WzViewerApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }
    
    fn load_file(&mut self, path: &PathBuf) {
        self.error_message = None;
        self.current_file = Some(path.clone());
        self.root_node = None;
        self.expanded_paths.clear();
        self.selected_path = None;
        self.current_image = None;
        self.image_path = None;
        
        // Determine file type by extension
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        
        let is_wz_file = extension == "wz";
        let is_img_file = extension == "img";
        
        if !is_wz_file && !is_img_file {
            self.error_message = Some("File must be a .wz or .img file".to_string());
            return;
        }
        
        // Get filename
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        
        let node_result = if is_wz_file {
            // Load as WZ file
            match WzFile::from_file(path, None, None, None) {
                Ok(wz_file) => {
                    // Create root node from WZ file
                    let node: WzNodeArc = WzNode::new(&name.into(), wz_file, None).into();
                    
                    // Parse the root node
                    let parse_result = {
                        let mut node_write = node.write().unwrap();
                        node_write.parse(&node)
                    };
                    
                    match parse_result {
                        Ok(_) => Ok(node),
                        Err(e) => Err(format!("Failed to parse WZ file: {:?}", e)),
                    }
                }
                Err(e) => Err(format!("Failed to load WZ file: {:?}", e)),
            }
        } else {
            // Load as IMG file (existing logic)
            match fs::read(path) {
                Ok(bytes) => {
                    // Guess IV
                    let wz_iv = match guess_iv_from_wz_img(&bytes) {
                        Some(iv) => iv,
                        None => {
                            self.error_message = Some("Unable to guess WZ version/IV from IMG file".to_string());
                            return;
                        }
                    };
                    
                    // Create reader
                    let reader = Arc::new(WzReader::new(bytes.clone()).with_iv(wz_iv));
                    
                    // Create WZ image
                    let wz_image = WzImage::new(&name.into(), 0, bytes.len(), &reader);
                    
                    // Create root node
                    let node: WzNodeArc = WzNode::new(&name.into(), wz_image, None).into();
                    
                    // Parse the root node
                    let parse_result = {
                        let mut node_write = node.write().unwrap();
                        node_write.parse(&node)
                    };
                    
                    match parse_result {
                        Ok(_) => Ok(node),
                        Err(e) => Err(format!("Failed to parse IMG file: {:?}", e)),
                    }
                }
                Err(e) => Err(format!("Failed to read file: {}", e)),
            }
        };
        
        match node_result {
            Ok(node) => {
                self.root_node = Some(node);
            }
            Err(e) => {
                self.error_message = Some(e);
            }
        }
    }
    
    fn render_tree(&mut self, ui: &mut egui::Ui, node: &WzNodeArc, node_path: &str) {
        let node_read = node.read().unwrap();
        let name = node_read.name.to_string();
        // node_path is the full path to this node (empty for root)
        let current_path = node_path.to_string();
        
        let is_expanded = self.expanded_paths.contains(&current_path);
        let node_type = match &node_read.object_type {
            WzObjectType::Directory(_) => "Directory",
            WzObjectType::Image(_) => "Image",
            WzObjectType::File(_) => "File",
            WzObjectType::MsFile(_) => "MsFile",
            WzObjectType::MsImage(_) => "MsImage",
            WzObjectType::Property(p) => match p {
                wz_reader::property::WzSubProperty::Convex => "Convex",
                wz_reader::property::WzSubProperty::Sound(_) => "Sound",
                wz_reader::property::WzSubProperty::PNG(_) => "PNG",
                wz_reader::property::WzSubProperty::Property => "Property",
            },
            WzObjectType::Value(v) => match v {
                wz_reader::property::WzValue::Null => "Null",
                wz_reader::property::WzValue::Short(_) => "Short",
                wz_reader::property::WzValue::Int(_) => "Int",
                wz_reader::property::WzValue::Long(_) => "Long",
                wz_reader::property::WzValue::Float(_) => "Float",
                wz_reader::property::WzValue::Double(_) => "Double",
                wz_reader::property::WzValue::String(_) => "String",
                wz_reader::property::WzValue::ParsedString(_) => "ParsedString",
                wz_reader::property::WzValue::Vector(_) => "Vector",
                wz_reader::property::WzValue::UOL(_) => "UOL",
                wz_reader::property::WzValue::RawData(_) => "RawData",
                wz_reader::property::WzValue::Video(_) => "Video",
                wz_reader::property::WzValue::Lua(_) => "Lua",
            },
        };
        
        let is_png = matches!(
            &node_read.object_type,
            WzObjectType::Property(wz_reader::property::WzSubProperty::PNG(_))
        );
        
        let has_children = !node_read.children.is_empty();
        
        // Render tree node in vertical layout to allow indentation
        ui.vertical(|ui| {
            // Render the node label in horizontal layout
            ui.horizontal(|ui| {
                let icon = if has_children {
                    if is_expanded { "▼" } else { "▶" }
                } else if is_png {
                    "🖼"
                } else {
                    "•"
                };
                
                let label_text = format!("{} {} [{}]", icon, name, node_type);
                let label = egui::Label::new(label_text)
                    .selectable(true)
                    .sense(egui::Sense::click());
                
                let response = ui.add(label);
                
            if response.clicked() {
                // Store the path (empty for root, relative path for children)
                self.selected_path = Some(current_path.clone());
                    
                    // Toggle expansion
                    if has_children {
                        if is_expanded {
                            self.expanded_paths.remove(&current_path);
                        } else {
                            self.expanded_paths.insert(current_path.clone());
                        }
                    }
                }
            });
            
            // Render children if expanded (now in vertical layout, so indent works)
            if is_expanded && has_children {
                ui.indent(current_path.clone(), |ui| {
                    // Re-read to get fresh children list after potential parsing
                    let child_names: Vec<String> = {
                        let node_read = node.read().unwrap();
                        node_read.children.keys().map(|n| n.to_string()).collect()
                    };
                    
                    for child_name in child_names {
                        // Get the child node
                        let child_node = {
                            let parent_read = node.read().unwrap();
                            // Convert String to WzNodeName explicitly
                            let node_name: WzNodeName = child_name.clone().into();
                            parent_read.children.get(&node_name).cloned()
                        };

                        if let Some(child_node) = child_node {
                            // Parse child node if needed - this is critical for Property nodes
                            {
                                let mut child_write = child_node.write().unwrap();
                                if let Err(_) = child_write.parse(&child_node) {
                                    // Continue even if parsing fails - might be a leaf node
                                }
                                drop(child_write);
                            }

                            // Build child path: if current is root (empty), use child name; otherwise append
                            let child_path = if current_path.is_empty() {
                                child_name.clone()
                            } else {
                                format!("{}/{}", current_path, child_name)
                            };

                            self.render_tree(ui, &child_node, &child_path);
                        }
                    }
                });
            }
        });
    }
    
    fn export_structure(&self, root: &WzNodeArc, output_path: &PathBuf) {
        let mut log_output = String::new();
        log_output.push_str(&format!(
            "=== WZ Structure ===\n\n"
        ));
        
        walk_node(root, true, &mut |n: &WzNodeArc| {
            let read = n.read().unwrap();
            let path = read.get_full_path();
            let type_name = match &read.object_type {
                WzObjectType::Directory(_) => "Directory",
                WzObjectType::Image(_) => "Image",
                WzObjectType::File(_) => "File",
                WzObjectType::MsFile(_) => "MsFile",
                WzObjectType::MsImage(_) => "MsImage",
                WzObjectType::Property(p) => match p {
                    wz_reader::property::WzSubProperty::Convex => "Convex",
                    wz_reader::property::WzSubProperty::Sound(_) => "Sound",
                    wz_reader::property::WzSubProperty::PNG(_) => "PNG",
                    wz_reader::property::WzSubProperty::Property => "Property",
                },
                WzObjectType::Value(v) => match v {
                    wz_reader::property::WzValue::Null => "Null",
                    wz_reader::property::WzValue::Short(_) => "Short",
                    wz_reader::property::WzValue::Int(_) => "Int",
                    wz_reader::property::WzValue::Long(_) => "Long",
                    wz_reader::property::WzValue::Float(_) => "Float",
                    wz_reader::property::WzValue::Double(_) => "Double",
                    wz_reader::property::WzValue::String(_) => "String",
                    wz_reader::property::WzValue::ParsedString(_) => "ParsedString",
                    wz_reader::property::WzValue::Vector(_) => "Vector",
                    wz_reader::property::WzValue::UOL(_) => "UOL",
                    wz_reader::property::WzValue::RawData(_) => "RawData",
                    wz_reader::property::WzValue::Video(_) => "Video",
                    wz_reader::property::WzValue::Lua(_) => "Lua",
                },
            };
            log_output.push_str(&format!("{} [{}]\n", path, type_name));
        });
        
        if let Err(e) = fs::write(output_path, log_output) {
            eprintln!("Failed to write export file: {}", e);
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("WZ/IMG File Viewer"),
        ..Default::default()
    };
    
    eframe::run_native(
        "WZ Viewer",
        options,
        Box::new(|cc| Box::new(WzViewerApp::new(cc))),
    )
}

