use std::sync::Arc;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzReader};

fn main() {
    let bytes = std::fs::read("MapHelper.img").expect("Failed to read MapHelper.img");

    let wz_iv = guess_iv_from_wz_img(&bytes).expect("Failed to guess IV");
    let byte_len = bytes.len();
    let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
    let cache_name_ref: wz_reader::WzNodeName = "MapHelper.img".into();
    let wz_image = WzImage::new(&cache_name_ref, 0, byte_len, &reader);
    let root_node: WzNodeArc = WzNode::new(&"MapHelper.img".into(), wz_image, None).into();

    root_node.write().unwrap().parse(&root_node).expect("Failed to parse");

    println!("=== MapHelper.img Portal Structure ===\n");

    // Navigate to portal node
    if let Ok(portal_node) = root_node.read().unwrap().at_path_parsed("portal") {
        let portal_read = portal_node.read().unwrap();

        // List all children under portal
        for (child_name, child_node) in portal_read.children.iter() {
            let child_read = child_node.read().unwrap();
            println!("portal/{}", child_name);

            // List sub-children
            for (sub_name, sub_node) in child_read.children.iter() {
                let sub_read = sub_node.read().unwrap();
                println!("  portal/{}/{}", child_name, sub_name);

                // List sub-sub-children (for nested structures)
                if !sub_read.children.is_empty() {
                    for (sub_sub_name, _) in sub_read.children.iter().take(5) {
                        println!("    portal/{}/{}/{}", child_name, sub_name, sub_sub_name);
                    }
                    if sub_read.children.len() > 5 {
                        println!("    ... and {} more", sub_read.children.len() - 5);
                    }
                }
            }
        }
    } else {
        println!("ERROR: No 'portal' node found in MapHelper.img!");
    }
}
