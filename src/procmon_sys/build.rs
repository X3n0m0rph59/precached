extern crate gcc;

fn main() {
    gcc::Build::new()
                .file("src/c/procmon.c")
                .include("src")
                .compile("procmon");
}
