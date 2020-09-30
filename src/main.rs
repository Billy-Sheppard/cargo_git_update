use colored::*;
use serde::Deserialize;
use std::{fs, process};
use structopt::StructOpt;

/// Cargo Git Update Flags
#[derive(StructOpt, Debug, Clone)]
#[structopt(name = "Cargo Git Update")]
struct Opt {
    /// Which dependency to try to update
    #[structopt(long = "dep")]
    dependency: String,
    /// If this flag is present, cargo update is not run.
    #[structopt(short = "u", long = "cargo_update")]
    update: bool,
}

type Result<T> = std::result::Result<T, anyhow::Error>;

#[derive(Debug, Deserialize)]
struct GitDep {
    git: String,
    tag: String,
}

fn main() -> Result<()> {
    env_logger::init();

    let mut flags = Opt::from_args();
    flags.update = !flags.update;

    log::debug!("{:#?}", flags);

    let toml = fs::read_to_string("Cargo.toml")?;
    let mut toml: toml::Value = toml::from_str(&toml)?;

    let toml_dep: GitDep = toml::from_str(
        &toml
            .get("dependencies")
            .ok_or_else(|| anyhow::anyhow!("No dependencies in this Cargo.toml."))?
            .get(&flags.dependency)
            .ok_or_else(|| anyhow::anyhow!("--dep flag does not match any dependency names."))?
            .to_string(),
    )?;

    let list_tags = process::Command::new("git")
        .args(&["ls-remote", "--tags", &toml_dep.git])
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&list_tags.stderr);
    let stderr: Vec<&str> = stderr.lines().collect();
    if !stderr.is_empty() {
        Err(anyhow::anyhow!("Git Error: {:#?}", stderr))
    } else {
        Ok(())
    }?;

    let stdout = String::from_utf8_lossy(&list_tags.stdout);
    let stdout: Vec<&str> = stdout.lines().collect();

    let mut tags: Vec<semver::Version> = stdout
        .into_iter()
        .filter_map(|t| {
            let s = t[51..].to_string();
            let s = s
                .split_at({
                    match s.rfind('v') {
                        Some(i) => i + 1,
                        None => 0,
                    }
                })
                .1;
            s.parse().ok()
        })
        .collect();
    tags.sort();
    println!(
        "    {} {} {} -> v{}",
        "Updating".bright_green().bold(),
        &flags.dependency,
        toml.get_mut("dependencies")
            .unwrap()
            .get_mut(&flags.dependency)
            .unwrap()
            .get_mut("tag")
            .ok_or_else(|| anyhow::anyhow!("There is no tag property on this dependency."))?
            .to_string()
            .replace("\"", ""),
        tags.last().unwrap()
    );

    if flags.update {
        let c_upt = process::Command::new("cargo")
            .arg("update")
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped())
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&c_upt.stderr);
        let _: Vec<&str> = stderr
            .lines()
            .map(|l| {
                if l.contains("Updating") {
                    let l2 = l.replace("Updating    ", "");
                    println!(
                        "    {} {}",
                        "Updating".bright_green().bold(),
                        l2.trim_start()
                    );
                } else {
                    println!("{}", l);
                }
                l
            })
            .collect();

        let stdout = String::from_utf8_lossy(&c_upt.stdout);
        let _: Vec<&str> = stdout
            .lines()
            .map(|l| {
                if l.contains("Updating") {
                    let l2 = l.replace("Updating    ", "");
                    println!(
                        "    {} {}",
                        "Updating".bright_green().bold(),
                        l2.trim_start()
                    );
                } else {
                    println!("{}", l);
                }
                l
            })
            .collect();
    };
    *toml
        .get_mut("dependencies")
        .unwrap()
        .get_mut(&flags.dependency)
        .unwrap()
        .get_mut("tag")
        .unwrap() = toml::Value::String(format!("v{}", tags.last().unwrap().to_string()));

    let package = toml.get("package").unwrap().to_string();
    *toml.get_mut("package").unwrap() = toml::Value::String("".to_string());
    fs::write("Cargo.toml", format!("[package]\n{}{}", package, toml.to_string().replace("package = \"\"\n", "")))?;
    Ok(())
}
