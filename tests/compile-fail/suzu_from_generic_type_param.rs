use suzunari_error::*;

#[suzunari_error]
#[suzu(display("generic from error"))]
struct GenericFromError<E: core::fmt::Debug + core::fmt::Display + 'static> {
    #[suzu(from)]
    source: E,
}

fn main() {}
