//! Snail launcher binary.
//!
//! This is a small launcher that discovers libpython and execs the main snail
//! binary with LD_PRELOAD set. This allows the main binary to be built without
//! linking against a specific libpython version.

use std::env;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")");

fn main() {
    if let Err(e) = run() {
        eprintln!("snail: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    // Get current args (skip the launcher name)
    let args: Vec<String> = env::args().skip(1).collect();

    // Handle --version / -v directly in the launcher
    if args.iter().any(|a| a == "--version" || a == "-v") {
        let lib_path = discover_libpython_path()?;
        println!("Snail: {}", VERSION);
        println!("Python: {}", lib_path);
        return Ok(());
    }

    // Discover the libpython path
    let lib_path = discover_libpython_path()?;

    // Get the path to the snail-core binary (same directory as launcher)
    let launcher_path =
        env::current_exe().map_err(|e| format!("Failed to get current exe: {}", e))?;
    let bin_dir = launcher_path
        .parent()
        .ok_or("Failed to get binary directory")?;
    let snail_path = bin_dir.join("snail-core");

    if !snail_path.exists() {
        return Err(format!("snail-core binary not found at {:?}", snail_path));
    }

    // Build the new LD_PRELOAD value
    let ld_preload = match env::var("LD_PRELOAD") {
        Ok(existing) => format!("{}:{}", lib_path, existing),
        Err(_) => lib_path,
    };

    // Exec the main snail binary with LD_PRELOAD set
    let err = Command::new(&snail_path)
        .args(&args)
        .env("LD_PRELOAD", ld_preload)
        .exec();

    // exec() only returns on error
    Err(format!("Failed to exec snail-core: {}", err))
}

/// Discover the path to libpython using Python itself.
fn discover_libpython_path() -> Result<String, String> {
    // Try using python3 to get the library path directly
    if let Ok(output) = Command::new("python3")
        .args([
            "-c",
            r#"
import sysconfig
import os

libdir = sysconfig.get_config_var('LIBDIR')
ldlibrary = sysconfig.get_config_var('LDLIBRARY')

if libdir and ldlibrary:
    full_path = os.path.join(libdir, ldlibrary)
    if os.path.exists(full_path):
        print(full_path)
    else:
        instsoname = sysconfig.get_config_var('INSTSONAME')
        if instsoname:
            full_path = os.path.join(libdir, instsoname)
            if os.path.exists(full_path):
                print(full_path)
"#,
        ])
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() && PathBuf::from(&path).exists() {
                return Ok(path);
            }
        }
    } else {
        // Command failed to execute, continue to fallback
    }

    // Fallback: try common library paths
    let common_paths = [
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib64",
        "/usr/lib",
        "/lib/x86_64-linux-gnu",
        "/lib64",
        "/lib",
    ];

    for dir in &common_paths {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("libpython3.")
                    && (name_str.ends_with(".so") || name_str.contains(".so."))
                    && !name_str.contains("libpython3.so")
                {
                    return Ok(entry.path().to_string_lossy().into_owned());
                }
            }
        }
    }

    Err("Could not find libpython3.*.so - ensure Python 3 is installed".to_string())
}
