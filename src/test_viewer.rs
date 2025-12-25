// Test program to verify viewer path handling and PNG loading
use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader, WzNodeCast};
use wz_reader::property::get_image;

fn test_path_resolution() {
    println!("=== Testing Path Resolution ===\n");

    // This is a mock test to demonstrate the correct path handling
    // In real usage, you would:
    // 1. Load an IMG file
    // 2. Parse the root node
    // 3. Navigate using paths like "Nexon/0" (not "Nexon/Nexon/0")

    println!("Expected path structure:");
    println!("Root node: \"\" (empty path)");
    println!("First child: \"Nexon\"");
    println!("Second level: \"Nexon/0\"");
    println!("Third level: \"Nexon/0/someProperty\"");
    println!("\nWith the fix, paths should NOT duplicate like \"Nexon/Nexon/0\"");
}

fn test_png_loading_steps() {
    println!("\n=== PNG Loading Steps ===\n");

    println!("To successfully load and display a PNG from an IMG file:");
    println!("1. Load the IMG file bytes");
    println!("2. Guess the IV/version using guess_iv_from_wz_img()");
    println!("3. Create a WzReader with the IV");
    println!("4. Create a WzImage and WzNode");
    println!("5. Parse the root node: node.write().unwrap().parse(&node)");
    println!("6. Navigate to a child: root.at_path_parsed(\"path/to/png\")");
    println!("7. Ensure the node is parsed: node.write().unwrap().parse(&node)");
    println!("8. Check if it's a PNG: node.try_as_png()");
    println!("9. Extract the image: get_image(&node)");
    println!("10. Convert to your display format (e.g., egui::ColorImage)");

    println!("\nCommon issues:");
    println!("- 'Node not parsed': The node or its parents weren't parsed");
    println!("- Wrong path: Check that paths don't have duplicated segments");
    println!("- Missing metadata: The PNG node needs to be parsed before accessing width/height");
}

fn main() {
    test_path_resolution();
    test_png_loading_steps();

    println!("\n=== Usage Example ===\n");
    println!("To test with a real IMG file:");
    println!("1. Run: cargo run --bin wz-viewer");
    println!("2. Click 'Open File' and select an IMG file");
    println!("3. Click on nodes in the tree to expand them");
    println!("4. Click on a PNG node to view it");
    println!("5. The right panel should show:");
    println!("   - Path to the node");
    println!("   - Type: PNG");
    println!("   - Size: WxH");
    println!("   - The actual image");

    println!("\nIf you see 'Node not found at path' errors:");
    println!("- The path handling fix should resolve this");
    println!("- Make sure to click on nodes to expand them first");
    println!("- The node tree should parse automatically as you expand");
}
