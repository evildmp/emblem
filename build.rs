include!("src/args.rs");

// use clap::CommandFactory;
use clap_complete::shells::Shell;
use clap_mangen::Man;
use std::error::Error;
use std::path::Path;
use std::{env, fs};

fn main() -> Result<(), Box<dyn Error>> {
    complgen()?;
    mangen()?;
    Ok(())
}

fn mangen() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=src/args.rs");
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_file = Path::new(&out_dir).join("em.1");

    let mut file = fs::File::create(dest_file)?;
    Man::new(Args::command()).render(&mut file)?;
    drop(file);
    // Man::new(Args::command()).render(&mut std::io::stdout())?;
    Ok(())
}

fn complgen() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=src/args.rs");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_dir = Path::new(&out_dir).join("completion");

    if !dest_dir.exists() {
        fs::create_dir(dest_dir.clone())?;
    }

    let shells = [
        Shell::Bash,
        Shell::Elvish,
        Shell::Fish,
        Shell::PowerShell,
        Shell::Zsh,
    ];
    for shell in shells {
        clap_complete::generate_to(shell, &mut Args::command(), "em", dest_dir.clone())?;
    }
    Ok(())
}
