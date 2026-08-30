use suzunari_error::*;

// Renaming is available, but through the macro's own attribute.
#[suzunari_error(serialize)]
#[serde(rename_all = "camelCase")]
#[suzu(display("type level"))]
struct RenameAllError {
    file_path: u32,
}

// Anything else at this level has no answer at all.
#[suzunari_error(serialize)]
#[serde(deny_unknown_fields)]
#[suzu(display("type level"))]
struct OtherError {
    value: u32,
}

fn main() {}
