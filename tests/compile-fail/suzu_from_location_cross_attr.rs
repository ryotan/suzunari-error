use suzunari_error::*;

#[suzunari_error]
#[suzu(display("bad"))]
struct BadError {
    #[suzu(from)]
    #[suzu(location)]
    source: Location,
}

fn main() {}
