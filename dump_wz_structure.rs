use std::cell::RefCell;
use std::env;
use std::fs;
use std::io::Write;
use std::sync::Arc;
use wz_reader::util::walk_node;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzObjectType, WzReader};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <input.img> <output.txt>", args[0]);
        eprintln!("Example: {} Login.img login_structure.txt", args[0]);
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    println!("Reading WZ file: {}", input_path);

    // Read the file
    let bytes = match fs::read(input_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

    println!("File size: {} bytes", bytes.len());

    // Guess IV
    let wz_iv = match guess_iv_from_wz_img(&bytes) {
        Some(iv) => iv,
        None => {
            eprintln!("Unable to guess WZ version/IV from file");
            std::process::exit(1);
        }
    };

    println!("Detected IV: {:?}", wz_iv);

    // Create reader (using from_buff for native builds)
    let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));

    // Get filename without extension
    let name = std::path::Path::new(input_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Create WZ image
    let wz_image = WzImage::new(&name.into(), 0, bytes.len(), &reader);

    // Create root node
    let node: WzNodeArc = WzNode::new(&name.into(), wz_image, None).into();

    println!("Parsing WZ structure...");

    // Collect structure (using RefCell for interior mutability)
    let log_output = RefCell::new(String::new());
    log_output.borrow_mut().push_str(&format!("=== WZ Structure for {} ===\n\n", input_path));

    walk_node(&node, true, &mut |n: &WzNodeArc| {
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
        log_output.borrow_mut().push_str(&format!("{} [{}]\n", path, type_name));
    });

    let log_output = log_output.into_inner();
    println!("Structure parsed ({} bytes of output)", log_output.len());

    // Write to file
    match fs::File::create(output_path) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(log_output.as_bytes()) {
                eprintln!("Error writing to file: {}", e);
                std::process::exit(1);
            }
            println!("✅ Structure saved to: {}", output_path);
        }
        Err(e) => {
            eprintln!("Error creating output file: {}", e);
            std::process::exit(1);
        }
    }
}
