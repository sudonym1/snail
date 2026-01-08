use std::env;
use std::fs;
use std::process::{Command, Stdio};

use clap::Parser;

use snail::{CompileMode, compile_snail_source_with_auto_print, format_snail_error};

#[cfg(debug_assertions)]
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")");

#[cfg(not(debug_assertions))]
const VERSION: &str = match option_env!("SNAIL_RELEASE_TAG") {
    Some(tag) if !tag.is_empty() => tag,
    _ => concat!("v", env!("CARGO_PKG_VERSION")),
};

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
    #[arg(short = 'P', long = "no-print")]
    no_auto_print: bool,

    /// Print version
    #[arg(short = 'v', long = "version")]
    version: bool,

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

    run_source(&input, !cli.no_auto_print)
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

fn run_source(input: &CliInput, auto_print: bool) -> Result<(), String> {
    let python_code = compile_snail_source_with_auto_print(&input.source, input.mode, auto_print)
        .map_err(|err| format_snail_error(&err, &input.filename))?;

    // Get Python interpreter from PYTHON env var, or default to python3
    let python_exe = env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());

    // Build the Python script with proper setup
    let script = format!(
        r#"import sys
sys.argv = {argv}
__file__ = {file}
__name__ = "__main__"

{code}"#,
        argv = format_python_list(&input.argv),
        file = format_python_string(&input.filename),
        code = python_code
    );

    // Execute Python code via subprocess
    let mut child = Command::new(&python_exe)
        .arg("-c")
        .arg(&script)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to execute {python_exe}: {e}"))?;

    let status = child
        .wait()
        .map_err(|e| format!("failed to wait for python process: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        std::process::exit(status.code().unwrap_or(1));
    }
}

fn format_python_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn format_python_list(items: &[String]) -> String {
    let formatted_items: Vec<String> = items.iter().map(|s| format_python_string(s)).collect();
    format!("[{}]", formatted_items.join(", "))
}
