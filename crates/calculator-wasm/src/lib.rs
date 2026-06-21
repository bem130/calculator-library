#![forbid(unsafe_code)]

pub fn protocol_version() -> (u16, u16) {
    let version = calculator_core::ProtocolVersion::CURRENT;
    (version.major, version.minor)
}
