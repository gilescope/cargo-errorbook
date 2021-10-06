#![feature(slice_group_by)]
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;
use std::process::Stdio;
use std::{io::Read, path::PathBuf};
use std::path::Path;

use cargo_metadata::diagnostic::DiagnosticCode;
use cargo_metadata::Message;

//TODO: NFT your error book in one click...
fn main() {
    if std::env::args_os().count() < 3 {
        println!("To Errorbook:");
        println!("E.g. cargo errorbook clippy or cargo errorbook check");
        std::process::exit(-1);
    }

    let args: Vec<_> = std::env::args_os().skip(2).collect();

    let target_dir = PathBuf::from("target/");

    let mut cmd = Command::new("cargo");
    cmd.args(args);
    cmd.stdout(Stdio::piped());
    cmd.arg("--message-format");
    cmd.arg("json");

    println!("running {:?}", &cmd);
    let out = cmd.output().unwrap();
    let mut errors = BTreeMap::new();
    let reader = std::io::BufReader::new(out.stdout.take(10_000_000));
    for message in cargo_metadata::Message::parse_stream(reader) {
        if let Message::CompilerMessage(msg) = message.unwrap() {
            let rendered = msg.message.rendered.unwrap();
            let first = rendered.lines().next().unwrap();
            let entry = errors
                .entry(crate_name(&msg.package_id.repr))
                .or_insert_with(Vec::new);
            entry.push(Error {
                name: first.to_owned(),
                rendered: rendered.clone(),
                code: msg.message.code,
            });
        }
    }
    for errors in errors.values_mut() {
        errors.sort_by(|a, b| a.name.cmp(&b.name));
        errors.retain(|e| !e.name.ends_with("warnings emitted"));
    }
    write_book(&errors, &target_dir);
}

fn crate_name(name: &str) -> String {
    let parts: Vec<_> = name.split(' ').collect();
    format!("{} {}", parts[0], parts[1])
}

fn crate_safe_file_name(name: &str) -> String {
    name.replace(" ", "_").replace(".", "_")
}

struct Error {
    name: String,
    rendered: String,
    code: Option<DiagnosticCode>,
}

fn write_book(errors: &BTreeMap<String, Vec<Error>>, target_dir: &Path) {
    println!("Found {} improvement points.", errors.len());
    if errors.is_empty() {
        return;
    }

    let data = r#"
    [book]
    title = "Yet Another ErrorBook"
    description = "A shrine to the hard work of Esteban and friends."
    authors = ["errorbook"]
    language = "en"
    
    [rust]
    edition = "2018"
    
    [output.html]
    mathjax-support = false
    default-theme = rust
    preferred-dark-theme = true
    
    [output.html.fold]
    enable = true
    level = 0
    
    [output.html.search]
    limit-results = 20
    use-boolean-and = true
    boost-title = 2
    boost-hierarchy = 2
    boost-paragraph = 1
    expand = true
    heading-split-level = 2
    "#;
    fs::create_dir_all(target_dir.join("errorbook/src")).unwrap();
    fs::write(target_dir.join("errorbook/Cargo.toml"), data).expect("Unable to write file");

    let mut summary = r##"
    # Summary

    "##
    .to_string();
    for (pkg, errors) in errors.iter() {
        let pkg_filename = crate_safe_file_name(pkg) + ".md";
        summary.push_str(&format!("\n   - [{}]({})", pkg, pkg_filename));
        let mut pkg_page = format!(r##"# {}"##, pkg);

        for (i, error) in errors.as_slice().group_by(|a,b|a.name == b.name).enumerate() {
            pkg_page.push_str(&format!(
                "\n   - [{} {}]({}{})",
                error[0].name.replace("[", "").replace("]", ""),
                error.len(),
                i,
                pkg_filename,
            ));
        }

        let pkg_pg_filename = format!("errorbook/src/{}", pkg_filename);
        fs::write(target_dir.join(pkg_pg_filename), pkg_page).expect("Unable to write file");

        for (i, error) in errors.as_slice().group_by(|a,b|a.name == b.name).enumerate() {
            summary.push_str(&format!(
                "\n      - [{}]({}{})",
                error[0].name.replace("[", "").replace("]", ""),
                i,
                pkg_filename
            ));

            let mut error_page = String::new();
            for err in error { 
                write_err(&mut error_page, err);
            }

            let error_pg_filename = format!("errorbook/src/{}{}", i, pkg_filename);
            fs::write(target_dir.join(error_pg_filename), error_page)
                .expect("Unable to write file");
        }
    }

    fs::write(target_dir.join("errorbook/src/SUMMARY.md"), summary).expect("Unable to write file");

    let mut compile = Command::new("mdbook");
    compile
        .arg("build")
        .current_dir(target_dir.join("errorbook"));
    compile
        .status()
        .unwrap_or_else(|_| panic!("{:?} failed. Maybe `cargo install mdbook`?", compile));

    let mut index = target_dir.join("errorbook/book/index.html");
    if let Ok(can) = std::fs::canonicalize(&index) {
        index = dbg!(can);
    }
    let url = dbg!(format!("file://{}", index.display()));
    webbrowser::open(&url).unwrap();
}

fn write_err(error_page: &mut String, err: &Error) {
    error_page.push_str(&format!(
        r##"# {}
```rust,noplaypen
{}
```

"##,
        err.name, err.rendered
    ));
    error_page.push('\n');
    if let Some(code) = &err.code {
        let mut explanation = err.rendered.clone();
        if let Some(explanation) = &code.explanation {
            error_page.push('\n');
            error_page.push_str("## Explanation:\n");
            error_page.push_str(&explanation.replace("\n\n```", "\n\n```rust"));
        }
        explanation.push(' ');

        let url = if code.code.starts_with('E') && code.code.len() <= 6 {
            format!("https://doc.rust-lang.org/error-index.html#{}", code.code)
        } else if let Some(ss) = dbg!(&explanation).rfind("https://") {
            explanation.chars().skip(ss).take_while(|ch| !ch.is_whitespace()).collect::<String>()      
        } else {
            format!("https://duckduckgo.com/?q=rust+{}", code.code)
        };
        error_page.push_str(&format!("\n\n( [Explain {} to me]({}) )\n\n", code.code, url));
    }    
}