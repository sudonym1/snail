use std::fs;
use std::io::{self, Write};

use clap::Parser;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};

use snail::{
    CompileMode, SnailError, compile_snail_source, compile_snail_source_with_auto_print,
    format_snail_error,
};

#[derive(Parser)]
#[command(
    name = "snail",
    about = "Snail programming language interpreter",
    override_usage = "snail [options] -f <file> [args]...\n       snail [options] <code> [args]...",
    trailing_var_arg = true
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
        let python = match compile_snail_source(&input.source, input.mode) {
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
