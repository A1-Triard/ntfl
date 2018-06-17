extern crate gcc;
extern crate pkg_config;

use pkg_config::Library;
use std::env;
use std::fs::{ File, remove_file };
use std::io::{ Write, Error, ErrorKind };
use std::path::{ Path, PathBuf };
use std::process::Command;

struct TempPath {
    path: PathBuf
}

impl TempPath {
    fn path(&self) -> &Path {
        &self.path.as_path()
    }
    fn new(path: PathBuf) -> TempPath {
        TempPath { path: path }
    }
}

impl Drop for TempPath {
    fn drop(&mut self) {
        match remove_file(self.path()) {
            Ok(()) => { },
            Err(ref e) if e.kind() == ErrorKind::NotFound => { },
            _ => eprintln!("Cannot delete temporary file {}.", &self.path().display())
        }
    }
}

fn find_library(names: &[&str]) -> Option<Library> {
    for name in names {
        if let Ok(lib) = pkg_config::probe_library(name) {
            return Some(lib);
        }
    }
    println!("cargo:rustc-link-lib={}", names.last().unwrap());
    None
}

fn main() {
    let ncurses_lib = if cfg!(target_os = "macos") {
        find_library(&["ncurses5", "ncurses"])
    } else {
        find_library(&["ncursesw5", "ncursesw"])
    };

    generate_chtype_rs(&ncurses_lib);
}

fn generate_chtype_rs(ncurses_lib: &Option<Library>) {
    let out_dir = env::var("OUT_DIR").expect("cannot get OUT_DIR");
    let src = TempPath::new(Path::new(&out_dir).join("size_of_chtype.c"));
    let bin = TempPath::new(Path::new(&out_dir).join("size_of_chtype.out"));

    let mut fp = File::create(&src.path).expect(&format!("cannot open {}", &src.path.display()));
    fp.write_all(b"#include <limits.h>
#include <stdio.h>
#include <ncurses.h>

int main(void) {
    printf(\"%zu\", sizeof(chtype) * CHAR_BIT);
    return 0;
}
").expect(&format!("cannot write into {}", &src.path().display()));

    let compiler = gcc::Build::new().get_compiler();
    let mut compile_cmd = Command::new(compiler.path());
    compile_cmd.arg(&src.path()).arg("-o").arg(&bin.path());
    if let &Some(ref lib) = ncurses_lib {
        for path in lib.include_paths.iter() {
            compile_cmd.arg("-I").arg(path);
        }
    }
    compile_cmd.status().expect("compilation failed");
    let chtype_size = Command::new(&bin.path()).output().unwrap();
    let dest_path = Path::new(&out_dir).join("chtype.rs");
    let mut f = File::create(&dest_path).unwrap();
    f.write_all(b"pub type chtype = u").unwrap();
    f.write_all(&chtype_size.stdout).unwrap();
    f.write_all(b";
").unwrap();
}
