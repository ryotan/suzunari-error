use suzunari_error::*;

#[suzunari_error(serialize)]
enum VariantError {
    #[serde(rename = "Renamed")]
    #[suzu(display("read"))]
    Read { value: u32 },
}

fn main() {}
