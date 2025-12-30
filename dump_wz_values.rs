use std::cell::RefCell;
use std::env;
use std::fs;
use std::sync::Arc;
use wz_reader::util::walk_node;
use wz_reader::version::guess_iv_from_wz_img;
use wz_reader::{WzImage, WzNode, WzNodeArc, WzObjectType, WzReader};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input.img> <output.txt>", args[0]);
        std::process::exit(1);
    }
    let input_path = &args[1];
    let output_path = &args[2];
    let bytes = fs::read(input_path).expect("Failed to read file");
    let wz_iv = guess_iv_from_wz_img(&bytes).expect("Unable to guess WZ version");
    let reader = Arc::new(WzReader::from_buff(&bytes).with_iv(wz_iv));
    let name = std::path::Path::new(input_path).file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
    let wz_image = WzImage::new(&name.into(), 0, bytes.len(), &reader);
    let node: WzNodeArc = WzNode::new(&name.into(), wz_image, None).into();
    let log_output = RefCell::new(String::new());
    walk_node(&node, true, &mut |n: &WzNodeArc| {
        let read = n.read().unwrap();
        let path = read.get_full_path();
        let value_str = match &read.object_type {
            WzObjectType::Value(v) => match v {
                wz_reader::property::WzValue::Short(val) => format!(" = {}", val),
                wz_reader::property::WzValue::Int(val) => format!(" = {}", val),
                wz_reader::property::WzValue::Long(val) => format!(" = {}", val),
                wz_reader::property::WzValue::Float(val) => format!(" = {}", val),
                wz_reader::property::WzValue::Double(val) => format!(" = {}", val),
                wz_reader::property::WzValue::Vector(vec) => format!(" = ({}, {})", vec.0, vec.1),
                _ => String::new(),
            },
            WzObjectType::Property(p) => match p {
                wz_reader::property::WzSubProperty::PNG(_) => " [PNG]".to_string(),
                _ => String::new(),
            },
            _ => String::new(),
        };
        log_output.borrow_mut().push_str(&format!("{}{}\n", path, value_str));
    });
    fs::write(output_path, log_output.into_inner()).expect("Failed to write output");
}
