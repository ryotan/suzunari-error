use suzunari_error::*;

#[suzunari_error(serialize)]
#[suzu(display("field level"))]
struct GetterError {
    // `getter` is accepted only inside a remote definition, so it would apply
    // to the generated one rather than to this type.
    #[serde(getter = "get")]
    value: u32,
    // Both are skipped in the definition, so an attribute here is ignored.
    #[serde(rename = "cause")]
    source: std::io::Error,
}

fn main() {}
