use std::collections::HashMap;
use std::fs;
use std::process::Command;
use std::process::Stdio;
use std::{io::Read, path::PathBuf};

use cargo_metadata::diagnostic::DiagnosticCode;
use cargo_metadata::*;

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
    let mut errors = HashMap::new();
    let reader = std::io::BufReader::new(out.stdout.take(10_000_000));
    for message in cargo_metadata::Message::parse_stream(reader) {
        if let Message::CompilerMessage(msg) = message.unwrap() {
            println!("{:#?}", msg);
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
    write_book(errors, target_dir);
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

fn write_book(errors: HashMap<String, Vec<Error>>, target_dir: PathBuf) {
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

        for (i, error) in errors.iter().enumerate() {
            pkg_page.push_str(&format!(
                "\n   - [{}]({}{})",
                error.name.replace("[", "").replace("]", ""),
                i,
                pkg_filename
            ));
        }

        let pkg_pg_filename = format!("errorbook/src/{}", pkg_filename);
        fs::write(target_dir.join(pkg_pg_filename), pkg_page).expect("Unable to write file");

        for (i, error) in errors.iter().enumerate() {
            summary.push_str(&format!(
                "\n      - [{}]({}{})",
                error.name.replace("[", "").replace("]", ""),
                i,
                pkg_filename
            ));
            let mut error_page = format!(
                r##"# {}
```rust,noplaypen
{}
```

"##,
                error.name, error.rendered
            );
            if let Some(code) = &error.code {
                if let Some(explanation) = &code.explanation {
                    error_page.push('\n');
                    error_page.push_str("## Explanation:\n");
                    error_page.push_str(&explanation.replace("\n\n```", "\n\n```rust"));
                }
                if code.code.starts_with('E') && code.code.len() <= 6 {
                    error_page.push_str(&format!("\n\n( [Explain {} to me](https://doc.rust-lang.org/error-index.html#{}) ).\n\n", code.code, code.code));
                } else {
                    error_page.push_str(&format!(
                        "\n\n( [Explain {} to me](https://duckduckgo.com/?q=rust+{}) ).\n\n",
                        code.code, code.code
                    ));
                }
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
