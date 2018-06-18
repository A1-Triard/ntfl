#![deny(warnings)]
extern crate gcc;
extern crate pkg_config;

use pkg_config::Library;
use std::env;
use std::fs::{ File, remove_file };
use std::io::{ Write, ErrorKind };
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

fn find_library<'a>(names: &[&'a str]) -> (&'a str, Option<Library>) {
    for name in names {
        if let Ok(lib) = pkg_config::probe_library(name) {
            return (name, Some(lib));
        }
    }
    let name = &names.last().unwrap();
    println!("cargo:rustc-link-lib={}", name);
    (name, None)
}

fn main() {
    let ncurses_lib = if cfg!(target_os = "macos") {
        find_library(&["ncurses5", "ncurses"])
    } else {
        find_library(&["ncursesw5", "ncursesw"])
    };
    generate_ncurses_link_rs(&ncurses_lib);
    generate_int_type_rs(false, "c_bool", "bool", b"#include <ncurses.h>
", &[&ncurses_lib]);
    generate_int_const_rs("c_int", "ERR", "d", b"#include <ncurses.h>
", &[&ncurses_lib]);
    generate_int_type_rs(false, "attr_t", "attr_t", b"#include <ncurses.h>
", &[&ncurses_lib]);
    generate_int_type_rs(false, "chtype", "chtype", b"#include <ncurses.h>
", &[&ncurses_lib]);
    generate_int_const_rs("c_uint", "KEY_CODE_YES", "d", b"#include <ncurses.h>
", &[&ncurses_lib]);
}

fn generate_ncurses_link_rs(ncurses_lib: &(&str, Option<Library>)) {
    let out_dir = env::var("OUT_DIR").expect("cannot get OUT_DIR");
    let ncurses_link_rs = Path::new(&out_dir).join("ncurses_link.rs");
    let mut f = File::create(&ncurses_link_rs).unwrap();
    f.write_all(b"#[link(name = \"").unwrap();
    f.write_all(ncurses_lib.0.as_bytes()).unwrap();
    f.write_all(b"\")]
extern { }
").unwrap();
}

fn generate_int_type_rs(is_signed: bool, rs_type_name: &str, type_name: &str, includes: &[u8], libs: &[&(&str, Option<Library>)]) {
    let size = from_c_code(type_name, &[ includes, b"#include <stdio.h>
#include <limits.h>

int main(void) {
    printf(\"%zu\", sizeof(", &type_name.as_bytes(), b") * CHAR_BIT);
    return 0;
}
" ], libs);
    generate_rs(rs_type_name, &[ b"#[allow(non_camel_case_types)]
type ", rs_type_name.as_bytes(), b" = ", if is_signed { b"i" } else { b"u" }, &size, b";
" ]);
}

fn generate_int_const_rs(type_name: &str, const_name: &str, printf: &str, includes: &[u8], libs: &[&(&str, Option<Library>)]) {
    let value = from_c_code(const_name, &[ includes, b"#include <stdio.h>

int main(void) {
    printf(\"%", printf.as_bytes(), b"\", ", const_name.as_bytes(), b");
    return 0;
}
" ], libs);
    generate_rs(const_name, &[ b"const ", const_name.as_bytes(), b": ", type_name.as_bytes(), b" = ", &value, b";
" ]);
}

fn generate_rs(name: &str, code: &[&[u8]]) {
    let out_dir = env::var("OUT_DIR").expect("cannot get OUT_DIR");
    let rs = Path::new(&out_dir).join(format!("{}.rs", name));
    let mut rs = File::create(&rs).unwrap();
    for code_part in code {
        rs.write_all(code_part).unwrap();
    }
}

fn from_c_code(name: &str, c_code: &[&[u8]], libs: &[&(&str, Option<Library>)]) -> Vec<u8> {
    let out_dir = env::var("OUT_DIR").expect("cannot get OUT_DIR");
    let src = TempPath::new(Path::new(&out_dir).join(&format!("{}.c", name)));
    let bin = TempPath::new(Path::new(&out_dir).join(&format!("{}.out", name)));

    let mut fp = File::create(&src.path).expect(&format!("cannot open {}", &src.path.display()));
    for c_code_part in c_code {
        fp.write_all(c_code_part).expect(&format!("cannot write into {}", &src.path().display()));
    }

    let compiler = gcc::Build::new().get_compiler();
    let mut compile_cmd = Command::new(compiler.path());
    compile_cmd.arg(&src.path()).arg("-o").arg(&bin.path());
    for &lib in libs {
        if let Some(ref lib) = lib.1 {
            for path in lib.include_paths.iter() {
                compile_cmd.arg("-I").arg(path);
            }
        }
    }
    compile_cmd.status().expect("compilation failed");
    let output = Command::new(&bin.path()).output().unwrap();
    output.stdout
}
