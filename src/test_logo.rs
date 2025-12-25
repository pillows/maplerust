use std::fs;
use std::sync::Arc;
use wz_reader::property::get_image;
use wz_reader::util::walk_node;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzNodeCast, WzReader};

fn main() {
    let path = "Logo.img";
    
    println!("Loading {}...", path);
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            return;
        }
    };
    
    println!("File size: {} bytes", bytes.len());
    
    // Guess IV
    let wz_iv = match guess_iv_from_wz_img(&bytes) {
        Some(iv) => iv,
        None => {
            eprintln!("Unable to guess WZ version/IV from file");
            return;
        }
    };
    
    println!("Detected IV: {:?}", wz_iv);
    
    // Create reader
    let reader = Arc::new(WzReader::new(bytes.clone()).with_iv(wz_iv));
    
    // Get filename
    let name = "Logo";
    
    // Create WZ image
    let wz_image = WzImage::new(&name.into(), 0, bytes.len(), &reader);
    
    // Create root node
    let node: WzNodeArc = WzNode::new(&name.into(), wz_image, None).into();
    
    // Parse the root node
    println!("Parsing root node...");
    if let Err(e) = node.write().unwrap().parse(&node) {
        eprintln!("Failed to parse root: {:?}", e);
        return;
    }
    
    println!("Root parsed successfully!");
    
    // Walk the tree and find PNGs
    let mut png_count = 0;
    walk_node(&node, true, &mut |n: &WzNodeArc| {
        let node_read = n.read().unwrap();
        
        // Check if it's a PNG using try_as_png
        if node_read.try_as_png().is_some() {
            png_count += 1;
            let path = node_read.get_full_path();
            println!("Found PNG #{} at path: {}", png_count, path);
            
            // Try to extract the image
            match get_image(n) {
                Ok(img) => {
                    println!("  Successfully extracted PNG: {}x{}", img.width(), img.height());
                }
                Err(e) => {
                    println!("  Failed to extract PNG: {:?}", e);
                }
            }
        }
    });
    
    println!("\nTotal PNGs found: {}", png_count);
    
    // Test specific paths from Logo.img - check what the root name is
    println!("\nTesting specific paths...");
    let root_read = node.read().unwrap();
    let root_name = root_read.name.to_string();
    println!("Root node name: {}", root_name);
    
    // List first few children
    println!("\nFirst 5 children of root:");
    for (i, (name, _)) in root_read.children.iter().take(5).enumerate() {
        println!("  {}: {}", i, name);
    }
    
    let test_paths = vec!["Nexon/0", "Wizet/0"];
    
    for test_path in test_paths {
        println!("\nTesting path: {}", test_path);
        
        // Try at_path_parsed
        match root_read.at_path_parsed(test_path) {
            Ok(target_node) => {
                let target_read = target_node.read().unwrap();
                println!("  ✓ Found node via at_path_parsed");
                println!("  Node type: {:?}", target_read.object_type);
                
                if target_read.try_as_png().is_some() {
                    println!("  Node is a PNG!");
                    match get_image(&target_node) {
                        Ok(img) => {
                            println!("  ✓ Successfully extracted: {}x{}", img.width(), img.height());
                        }
                        Err(e) => {
                            println!("  ✗ Failed to extract: {:?}", e);
                        }
                    }
                } else {
                    println!("  Node is NOT a PNG (type: {:?})", target_read.object_type);
                }
            }
            Err(e) => {
                println!("  ✗ at_path_parsed failed: {:?}", e);
                
                // Try at_path as fallback
                if let Some(target_node) = root_read.at_path(test_path) {
                    let target_read = target_node.read().unwrap();
                    println!("  ✓ Found node via at_path (fallback)");
                    println!("  Node type: {:?}", target_read.object_type);
                    
                    if target_read.try_as_png().is_some() {
                        println!("  Node is a PNG!");
                        match get_image(&target_node) {
                            Ok(img) => {
                                println!("  ✓ Successfully extracted: {}x{}", img.width(), img.height());
                            }
                            Err(e) => {
                                println!("  ✗ Failed to extract: {:?}", e);
                            }
                        }
                    }
                } else {
                    println!("  ✗ at_path also failed - node not found");
                }
            }
        }
    }
}

