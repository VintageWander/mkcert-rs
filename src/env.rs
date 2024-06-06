use dotenvy::var;
use rcgen::DnValue;

pub fn home_dir() -> String {
    std::env::var_os("HOME")
        .expect("No HOME environment variable set")
        .into_string()
        .expect("Invalid HOME environment variable")
}
pub fn common_name() -> String {
    var("COMMON_NAME").unwrap_or("Mkcert Development Certificate".into())
}
pub fn locality() -> DnValue {
    DnValue::Utf8String(var("LOCALITY").unwrap_or("San Francisco".into()))
}
pub fn country() -> DnValue {
    DnValue::Utf8String(var("COUNTRY").unwrap_or("US".into()))
}

pub fn org_unit() -> DnValue {
    DnValue::Utf8String(var("ORG_UNIT").unwrap_or("Development".into()))
}
pub fn org_name() -> DnValue {
    DnValue::Utf8String(var("ORG_NAME").unwrap_or("Mkcert".into()))
}
