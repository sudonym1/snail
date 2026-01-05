use std::fs;
use std::io::{self, Write};

use clap::Parser;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};

use snail::{CompileMode, SnailError, compile_snail_source_with_auto_print, format_snail_error};

const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")");

#[derive(Parser)]
#[command(
    name = "snail",
    version = VERSION,
    about = "Snail programming language interpreter",
    override_usage = "snail [options] -f <file> [args]...\n       snail [options] <code> [args]...",
    trailing_var_arg = true,
    disable_version_flag = true
)]
struct Cli {
    /// Run a source file instead of a oneliner
    #[arg(short = 'f', value_name = "file")]
    file: Option<String>,

    /// Run code in awk mode (pattern/action over input)
    #[arg(short = 'a', long = "awk")]
    awk: bool,

    /// Print generated python
    #[arg(short = 'p', long = "python")]
    python: bool,

    /// Disable auto-printing of last expression result
    #[arg(short = 'P')]
    no_auto_print: bool,

    /// Print version
    #[arg(short = 'v', long = "version")]
    version: bool,

    /// Self-update to the latest release from GitHub
    #[arg(long = "update")]
    update: bool,

    /// Input file and arguments passed to the script
    #[arg(allow_hyphen_values = true)]
    args: Vec<String>,
}

struct CliInput {
    filename: String,
    source: String,
    argv: Vec<String>,
    mode: CompileMode,
}

fn main() {
    if let Err(err) = run() {
        if !err.is_empty() {
            eprintln!("{err}");
        }
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();

    if cli.version {
        println!("{VERSION}");
        return Ok(());
    }

    if cli.update {
        return self_update();
    }

    let mode = if cli.awk {
        CompileMode::Awk
    } else {
        CompileMode::Snail
    };

    let input = if let Some(fpath) = cli.file {
        let source = read_source(&fpath)?;
        CliInput {
            filename: fpath.clone(),
            source,
            argv: build_argv(&fpath, cli.args),
            mode,
        }
    } else if let Some((source, args)) = cli.args.split_first() {
        CliInput {
            filename: "<cmd>".to_string(),
            source: source.to_string(),
            argv: build_argv("--", args.to_vec()),
            mode,
        }
    } else {
        return Err("no input provided".to_string());
    };

    if cli.python {
        let python = match compile_snail_source_with_auto_print(
            &input.source,
            input.mode,
            !cli.no_auto_print,
        ) {
            Ok(python) => python,
            Err(err) => return Err(format_snail_error(&err, &input.filename)),
        };
        println!("{python}");
        return Ok(());
    }

    match run_source(&input, !cli.no_auto_print) {
        Ok(()) => Ok(()),
        Err(CliError::Snail(err)) => Err(format_snail_error(&err, &input.filename)),
        Err(CliError::Python(err)) => {
            Python::with_gil(|py| err.print(py));
            Err(String::new())
        }
    }
}

fn read_source(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path))
}

fn build_argv(argv0: &str, extra: Vec<String>) -> Vec<String> {
    let mut argv = Vec::with_capacity(1 + extra.len());
    argv.push(argv0.to_string());
    argv.extend(extra);
    argv
}

enum CliError {
    Snail(SnailError),
    Python(PyErr),
}

fn run_source(input: &CliInput, auto_print: bool) -> Result<(), CliError> {
    let python = compile_snail_source_with_auto_print(&input.source, input.mode, auto_print)
        .map_err(CliError::Snail)?;

    Python::with_gil(|py| -> Result<(), CliError> {
        let builtins = PyModule::import_bound(py, "builtins").map_err(CliError::Python)?;
        let code = builtins
            .getattr("compile")
            .and_then(|compile| compile.call1((python, &input.filename, "exec")))
            .map_err(CliError::Python)?;

        let globals = PyDict::new_bound(py);
        globals
            .set_item("__name__", "__main__")
            .map_err(CliError::Python)?;
        globals
            .set_item("__file__", &input.filename)
            .map_err(CliError::Python)?;

        let sys = PyModule::import_bound(py, "sys").map_err(CliError::Python)?;
        let argv = PyList::new_bound(py, &input.argv);
        sys.setattr("argv", argv).map_err(CliError::Python)?;

        builtins
            .getattr("exec")
            .and_then(|exec| exec.call1((code, &globals, &globals)))
            .map_err(CliError::Python)?;

        // Ensure buffered output is flushed for non-interactive runs.
        flush_python_io(&sys);

        Ok(())
    })
}

fn flush_python_io(sys: &Bound<'_, PyModule>) {
    for name in ["stdout", "stderr", "__stdout__", "__stderr__"] {
        if let Ok(stream) = sys.getattr(name) {
            let _ = stream.call_method0("flush");
        }
    }
    let _ = io::stdout().flush();
    let _ = io::stderr().flush();
}

fn self_update() -> Result<(), String> {
    use serde::Deserialize;
    use std::env;
    use std::fs::File;
    use std::io::copy;

    #[derive(Deserialize)]
    struct Release {
        tag_name: String,
        assets: Vec<Asset>,
    }

    #[derive(Deserialize)]
    struct Asset {
        name: String,
        browser_download_url: String,
    }

    println!("Checking for latest release...");

    // Get the latest release from GitHub
    let client = reqwest::blocking::Client::builder()
        .user_agent("snail-updater")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let release: Release = client
        .get("https://api.github.com/repos/sudonym1/snail/releases/latest")
        .send()
        .map_err(|e| format!("Failed to fetch release info: {}", e))?
        .json()
        .map_err(|e| format!("Failed to parse release info: {}", e))?;

    println!("Latest release: {}", release.tag_name);

    // Determine the asset name for the current platform
    let asset_name = if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        "snail-linux-x86_64"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        "snail-macos-x86_64"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "snail-macos-aarch64"
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        "snail-windows-x86_64.exe"
    } else {
        return Err(format!(
            "Unsupported platform: {}-{}",
            env::consts::OS,
            env::consts::ARCH
        ));
    };

    // Find the asset
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| format!("Asset {} not found in release", asset_name))?;

    println!("Downloading {}...", asset.name);

    // Download the binary
    let response = client
        .get(&asset.browser_download_url)
        .send()
        .map_err(|e| format!("Failed to download binary: {}", e))?;

    // Get the current executable path
    let current_exe = env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {}", e))?;

    // Create a temporary file
    let temp_path = current_exe.with_extension("tmp");
    let mut temp_file = File::create(&temp_path)
        .map_err(|e| format!("Failed to create temporary file: {}", e))?;

    // Write the downloaded binary to the temp file
    let content = response.bytes().map_err(|e| format!("Failed to read response: {}", e))?;
    copy(&mut content.as_ref(), &mut temp_file)
        .map_err(|e| format!("Failed to write binary: {}", e))?;

    drop(temp_file);

    // Make the temp file executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&temp_path)
            .map_err(|e| format!("Failed to get file metadata: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&temp_path, perms)
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    // Replace the current executable with the new one
    fs::rename(&temp_path, &current_exe)
        .map_err(|e| format!("Failed to replace executable: {}", e))?;

    println!("Successfully updated to {}", release.tag_name);
    Ok(())
}
