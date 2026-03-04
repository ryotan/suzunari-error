use suzunari_error::*;

#[suzunari_error]
enum GenericFromEnum<E: core::fmt::Debug + core::fmt::Display + 'static> {
    #[suzu(display("generic from enum"))]
    Variant {
        #[suzu(from)]
        source: E,
    },
}

fn main() {}
