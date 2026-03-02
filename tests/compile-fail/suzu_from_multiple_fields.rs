use suzunari_error::suzunari_error;

#[derive(Debug)]
struct LibErrorA(String);
impl std::fmt::Display for LibErrorA {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug)]
struct LibErrorB(String);
impl std::fmt::Display for LibErrorB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[suzunari_error]
#[suzu(display("error"))]
struct MyError {
    #[suzu(from)]
    source: LibErrorA,
    #[suzu(from)]
    other: LibErrorB,
}

fn main() {}
