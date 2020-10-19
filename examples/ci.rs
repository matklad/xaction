use std::{env, process};

use xaction::{cargo_toml, cmd, git, section, Result};

fn main() {
    if let Err(err) = try_main() {
        eprintln!("{}", err);
        process::exit(1);
    }
}

fn try_main() -> Result<()> {
    let cargo_toml = cargo_toml()?;

    {
        let _s = section("BUILD");
        cmd!("cargo test --workspace --no-run").run()?;
    }

    {
        let _s = section("TEST");
        cmd!("cargo test --workspace --no-run").run()?;
    }

    let version = cargo_toml.version()?;
    let tag = format!("v{}", version);
    let dry_run = env::var("CI").is_err()
        || git::tag_list()?.contains(&tag)
        || git::current_branch()? != "master";
    xaction::set_dry_run(dry_run);

    {
        let _s = section("PUBLISH");
        cargo_toml.publish()?;
        git::tag(&tag)?;
        git::push_tags()?;
    }
    Ok(())
}
