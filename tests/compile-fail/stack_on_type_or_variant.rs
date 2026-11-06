use suzunari_error::*;

#[suzunari_error]
#[stack(location)]
struct StructError {}

#[suzunari_error]
#[stack(location)]
enum EnumError {
    EnumFailed {},
}

#[suzunari_error]
enum VariantError {
    #[stack(location)]
    VariantFailed {},
}

fn main() {}
