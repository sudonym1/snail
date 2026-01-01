use std::env;
use std::fs;
use std::io::{self, Write};

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};

use snail::{CompileMode, SnailError, compile_snail_source_with_mode, format_snail_error};

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
    let mut args = env::args().skip(1);
    let mut code = None;
    let mut filename = None;
    let mut argv = Vec::new();
    let mut print_python = false;
    let mut passthrough = false;
    let mut awk_mode = false;

    while let Some(arg) = args.next() {
        if passthrough {
            argv.push(arg);
            continue;
        }

        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            "-p" | "--python" => {
                print_python = true;
            }
            "-a" | "--awk" => {
                awk_mode = true;
            }
            "-c" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing argument for -c".to_string())?;
                code = Some(value);
            }
            "--" => passthrough = true,
            _ if arg.starts_with('-') => {
                return Err(format!("unknown option: {arg}"));
            }
            _ => {
                if filename.is_some() {
                    return Err("multiple input files provided".to_string());
                }
                filename = Some(arg);
            }
        }
    }

    let input = match (code, filename) {
        (Some(_), Some(_)) => return Err("use -c for code or a file path, not both".to_string()),
        (None, None) => {
            print_help();
            return Ok(());
        }
        (Some(source), None) => CliInput {
            filename: "<cmd>".to_string(),
            source,
            argv: build_argv("-c", argv),
            mode: if awk_mode {
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
                argv: build_argv(&path, argv),
                mode: if awk_mode {
                    CompileMode::Awk
                } else {
                    CompileMode::Auto
                },
            }
        }
    };

    if print_python {
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

fn print_help() {
    let mut out = io::stderr();
    let _ = writeln!(out, "usage: snail [options] <file>");
    let _ = writeln!(out, "       snail -c <code>");
    let _ = writeln!(out);
    let _ = writeln!(out, "options:");
    let _ = writeln!(out, "  -c <code>    run a one-liner");
    let _ = writeln!(
        out,
        "  -a, --awk    run code in awk mode (pattern/action over input)"
    );
    let _ = writeln!(out, "  -p, --python output translated Python and exit");
    let _ = writeln!(out, "  -h, --help   show this help");
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
