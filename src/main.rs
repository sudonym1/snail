use std::fs;
use std::io::{self, Write};

use clap::Parser;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};

use snail::{CompileMode, SnailError, compile_snail_source_with_mode, format_snail_error};

#[derive(Parser)]
#[command(
    name = "snail",
    about = "Snail programming language interpreter",
    override_usage = "snail [options] <file>\n       snail -c <code>"
)]
struct Cli {
    /// Run a one-liner
    #[arg(short = 'c', value_name = "code", conflicts_with = "file")]
    code: Option<String>,

    /// Run code in awk mode (pattern/action over input)
    #[arg(short = 'a', long = "awk")]
    awk: bool,

    /// Output translated Python and exit
    #[arg(short = 'p', long = "python")]
    python: bool,

    /// Input file
    #[arg(conflicts_with = "code")]
    file: Option<String>,

    /// Arguments passed to the script
    #[arg(last = true)]
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

    let input = match (cli.code, cli.file) {
        (Some(source), None) => CliInput {
            filename: "<cmd>".to_string(),
            source,
            argv: build_argv("-c", cli.args),
            mode: if cli.awk {
                CompileMode::Awk
            } else {
                CompileMode::Auto
            },
        },
        (None, Some(path)) => {
            let source = read_source(&path)?;
            CliInput {
                filename: path.clone(),
                source,
                argv: build_argv(&path, cli.args),
                mode: if cli.awk {
                    CompileMode::Awk
                } else {
                    CompileMode::Auto
                },
            }
        }
        (None, None) => {
            // No input provided - clap will have shown help or we need to show usage
            return Err("no input provided; use -c <code> or provide a file".to_string());
        }
        (Some(_), Some(_)) => unreachable!(), // clap handles this conflict
    };

    if cli.python {
        let python = match compile_snail_source_with_mode(&input.source, input.mode) {
            Ok(python) => python,
            Err(err) => return Err(format_snail_error(&err, &input.filename)),
        };
        println!("{python}");
        return Ok(());
    }

    match run_source(&input) {
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

fn run_source(input: &CliInput) -> Result<(), CliError> {
    let python =
        compile_snail_source_with_mode(&input.source, input.mode).map_err(CliError::Snail)?;

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
