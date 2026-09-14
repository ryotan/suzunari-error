use suzunari_error::*;

#[suzunari_error(serialize(unknown = "x"))]
#[suzu(display("option"))]
struct UnknownOptionError {
    value: u32,
}

#[suzunari_error(serialize(rename_all = 3))]
#[suzu(display("option"))]
struct NotAStringError {
    value: u32,
}

fn main() {}
