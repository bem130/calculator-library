#![forbid(unsafe_code)]

fn main() {
    let version = calculator_core::ProtocolVersion::CURRENT;
    println!("calculator-wasi {}.{}", version.major, version.minor);
}
