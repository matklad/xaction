use std::{
    env,
    path::PathBuf,
    sync::atomic::AtomicBool,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

pub use xshell::*;

pub type Error = Box<dyn std::error::Error>;
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub fn section(name: &'static str) -> Section {
    Section::new(name)
}

pub fn push_rustup_toolchain(name: &str) -> Pushenv {
    pushenv("RUSTUP_TOOLCHAIN", name)
}

static DRY_RUN: AtomicBool = AtomicBool::new(false);
pub fn set_dry_run(yes: bool) {
    DRY_RUN.store(yes, Ordering::Relaxed)
}
fn dry_run() -> Option<&'static str> {
    let dry_run = DRY_RUN.load(Ordering::Relaxed);
    if dry_run {
        Some("--dry-run")
    } else {
        None
    }
}

pub fn cargo_toml() -> Result<CargoToml> {
    let cwd = cwd()?;
    let path = cwd.join("Cargo.toml");
    let contents = read_file(&path)?;
    Ok(CargoToml { path, contents })
}

pub struct CargoToml {
    path: PathBuf,
    contents: String,
}

impl CargoToml {
    pub fn version(&self) -> Result<&str> {
        self.get("version")
    }

    fn get(&self, field: &str) -> Result<&str> {
        for line in self.contents.lines() {
            let words = line.split_ascii_whitespace().collect::<Vec<_>>();
            match words.as_slice() {
                [n, "=", v, ..] if n.trim() == field => {
                    assert!(v.starts_with('"') && v.ends_with('"'));
                    return Ok(&v[1..v.len() - 1]);
                }
                _ => (),
            }
        }
        Err(format!("can't find `{}` in {}", field, self.path.display()))?
    }

    pub fn publish(&self) -> Result<()> {
        let token = env::var("CRATES_IO_TOKEN").unwrap_or("no token".to_string());
        let dry_run = dry_run();
        cmd!("cargo publish --token {token} {dry_run...}").run()?;
        Ok(())
    }
    pub fn publish_all(&self, dirs: &[&str]) -> Result<()> {
        let token = env::var("CRATES_IO_TOKEN").unwrap_or("no token".to_string());
        if dry_run().is_none() {
            for &dir in dirs {
                for _ in 0..20 {
                    std::thread::sleep(Duration::from_secs(10));
                    if cmd!("cargo publish --manifest-path {dir}'/Cargo.toml' --token {token} --dry-run").run().is_ok()
                    {
                        break;
                    }
                }
                cmd!("cargo publish --manifest-path {dir}'/Cargo.toml' --token {token}").run()?;
            }
        }
        Ok(())
    }
}

pub mod git {
    use xshell::cmd;

    use super::{dry_run, Result};

    pub fn current_branch() -> Result<String> {
        let res = cmd!("git branch --show-current").read()?;
        Ok(res)
    }

    pub fn tag_list() -> Result<Vec<String>> {
        let tags = cmd!("git tag --list").read()?;
        let res = tags.lines().map(|it| it.trim().to_string()).collect();
        Ok(res)
    }

    pub fn has_tag(tag: &str) -> Result<bool> {
        let res = tag_list()?.iter().any(|it| it == tag);
        Ok(res)
    }

    pub fn tag(tag: &str) -> Result<()> {
        if dry_run().is_some() {
            return Ok(());
        }
        cmd!("git tag {tag}").run()?;
        Ok(())
    }

    pub fn push_tags() -> Result<()> {
        // `git push --tags --dry-run` exists, but it will fail with permissions
        // error for forks.
        if dry_run().is_some() {
            return Ok(());
        }

        cmd!("git push --tags").run()?;
        Ok(())
    }
}

pub struct Section {
    name: &'static str,
    start: Instant,
}

impl Section {
    fn new(name: &'static str) -> Section {
        println!("::group::{}", name);
        let start = Instant::now();
        Section { name, start }
    }
}

impl Drop for Section {
    fn drop(&mut self) {
        eprintln!("{}: {:.2?}", self.name, self.start.elapsed());
        println!("::endgroup::");
    }
}
