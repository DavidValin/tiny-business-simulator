// Command-line interface for the product-definition parser / simulator.
//
// Usage:
//   ./tiny-business-simulator [--lang <lang_code>] <root_folder>
//          Launch the interactive product simulator menu.
//   ./tiny-business-simulator [--lang <lang_code>] --list <root_folder>
//          Parse and print every product definition.
//
// `<root_folder>` is a folder containing one or more `.txt` product
// definition files (see `parser.rs` for the supported syntax). `<lang_code>`
// is one of: en (default), es, zh, de, ru, fr.

#[allow(dead_code)]
mod lang;
mod parser;
#[allow(dead_code)]
mod simulator;
mod tui;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use lang::Lang;
use parser::{parse_content, ProductDefinition};

fn print_help(prog: &str, lang: &Lang) {
    let d = lang.dict();
    println!("\ntiny-business-simulator {} - https://crates.io/crates/tiny-business-simulator", env!("CARGO_PKG_VERSION"));
    println!("by David Valin <hola@davidvalin.com> - www.davidvalin.com\n");
    println!(" {}", d.usage_label);
    println!(" {}", lang::fmt(d.usage_interactive, &[prog]));
    println!(" {}", lang::fmt(d.usage_list, &[prog]));
    println!(" {}\n", lang::fmt(d.usage_lang, &[prog]));
}

fn print_all_products(folder: &Path, lang: &Lang) {
    let d = lang.dict();
    let txt_files = simulator::collect_txt_files(folder);
    if txt_files.is_empty() {
        println!("{}", lang::fmt(d.no_txt_files, &[&folder.display().to_string()]));
        return;
    }

    let mut products: Vec<(PathBuf, ProductDefinition)> = Vec::new();
    for file in &txt_files {
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                simulator::print_error(&lang::fmt(
                    d.cannot_read_file,
                    &[&file.display().to_string(), &e.to_string()],
                ));
                continue;
            }
        };
        match parse_content(&content, lang) {
            Ok(product) => products.push((file.clone(), product)),
            Err(errors) => {
                for err in &errors {
                    simulator::print_error(&lang::fmt(
                        d.file_colon_msg,
                        &[&file.display().to_string(), err],
                    ));
                }
            }
        }
    }

    println!();
    println!("{}", lang::fmt(d.parsed_products_header, &[&products.len().to_string()]));
    if products.is_empty() {
        println!("{}", d.no_valid_products);
    }
    for (file, p) in &products {
        println!();
        println!("{}", lang::fmt(d.file_label, &[&file.display().to_string()]));
        print!("{}", p.display(lang));
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let prog = args.get(0).map(|s| s.as_str()).unwrap_or("tiny-business-simulator");

    let mut lang_code: Option<String> = None;
    let mut list_mode = false;
    let mut folder: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        if a == "-h" || a == "--help" {
            print_help(prog, &Lang::DEFAULT);
            std::process::exit(0);
        } else if a == "--list" {
            list_mode = true;
        } else if a == "--lang" {
            if i + 1 >= args.len() {
                eprintln!("{}", lang::fmt(lang::EN.unknown_lang, &[""]));
                print_help(prog, &Lang::En);
                std::process::exit(1);
            }
            lang_code = Some(args[i + 1].clone());
            i += 1;
        } else if let Some(code) = a.strip_prefix("--lang=") {
            lang_code = Some(code.to_string());
        } else {
            folder = Some(a.clone());
        }
        i += 1;
    }

    let lang = match lang_code {
        Some(c) => match Lang::from_code(&c) {
            Some(l) => l,
            None => {
                eprintln!("{}", lang::fmt(lang::EN.unknown_lang, &[&c]));
                std::process::exit(1);
            }
        },
        None => Lang::DEFAULT,
    };

    let folder = match folder {
        Some(f) => f,
        None => {
            print_help(prog, &lang);
            std::process::exit(1);
        }
    };

    let path = Path::new(&folder);
    if !path.is_dir() {
        eprintln!("{}", lang::fmt(lang.dict().not_a_directory, &[&folder]));
        std::process::exit(1);
    }

    if list_mode {
        print_all_products(path, &lang);
    } else {
        tui::run(path, &lang);
    }
}
