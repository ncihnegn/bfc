#![feature(plugin)]
#![plugin(quickcheck_macros)]

extern crate llvm_sys;
extern crate itertools;
extern crate quickcheck;
extern crate rand;

use std::env;
use std::fs::File;
use std::io::Write;
use std::io::prelude::Read;
use std::path::Path;

mod bfir;
mod llvm;
mod optimize;
mod bounds;

#[cfg(test)]
mod optimize_tests;
mod llvm_tests;

/// Read the contents of the file at path, and return a string of its
/// contents.
fn slurp(path: &str) -> Result<String, std::io::Error> {
    let mut file = try!(File::open(path));
    let mut contents = String::new();
    try!(file.read_to_string(&mut contents));
    Ok(contents)
}

/// Convert "foo.bf" to "foo.ll".
fn ll_file_name(bf_file_name: &str) -> String {
    let mut name_parts: Vec<_> = bf_file_name.split('.').collect();
    let parts_len = name_parts.len();
    if parts_len > 1 {
        name_parts[parts_len - 1] = "ll";
    } else {
        name_parts.push("ll");
    }

    name_parts.connect(".")
}

#[cfg_attr(test, allow(dead_code))]
fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() > 1 {

        // TODO: proper options parsing
        let dump_bf_ir = args.len() > 2 && args[2] == "--dump-bf-ir";
        let dump_llvm = args.len() > 2 && args[2] == "--dump-llvm";
        
        let ref file_path = args[1];
        match slurp(&file_path) {
            Ok(src) => {
                let mut instrs;
                match bfir::parse(&src) {
                    Ok(instrs_) => {
                        instrs = instrs_;
                    },
                    Err(message) => {
                        println!("{}", message);
                        std::process::exit(1);
                    }
                }
                // TODO: allow users to specify optimisation level.
                instrs = optimize::optimize(instrs);

                if dump_bf_ir {
                    for instr in &instrs {
                        println!("{}", instr);
                    }
                    return
                }

                let num_cells = bounds::highest_cell_index(&instrs) + 1;

                let llvm_ir_raw;
                unsafe {
                    llvm_ir_raw = llvm::compile_to_ir(&file_path, &instrs, num_cells);
                }

                if dump_llvm {
                    let llvm_ir = String::from_utf8_lossy(llvm_ir_raw.as_bytes());
                    println!("{}", llvm_ir);
                } else {
                    // TODO: write to a temp file then call llc.
                    let bf_name = Path::new(file_path).file_name().unwrap();
                    let ll_name = ll_file_name(bf_name.to_str().unwrap());
                    match File::create(&ll_name) {
                        Ok(mut f) => {
                            let _ = f.write(llvm_ir_raw.as_bytes());
                            println!("Wrote {}", ll_name);
                        }
                        Err(e) => {
                            println!("Could not create file: {}", e);
                            std::process::exit(1);
                        }
                    }
                }

            }
            Err(e) => {
                println!("Could not open file: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        println!("Usage: {} <BF source file> [options...]", args[0]);
        println!("Examples:");
        println!("  {} foo.bf", args[0]);
        println!("  {} foo.bf --dump-bf-ir", args[0]);
        println!("  {} foo.bf --dump-llvm", args[0]);
        std::process::exit(1);
    }
    
}
