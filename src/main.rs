use std::collections::VecDeque;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::Parser;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};

use snail::{
    CompileMode, SnailError, compile_snail_source_with_mode, format_snail_error,
    format_snail_source, unified_diff,
};

#[derive(Parser)]
#[command(
    name = "snail",
    about = "Snail programming language interpreter",
    override_usage = "snail [options] <file> [args]...\n       snail [options] -c <code> [args]...",
    trailing_var_arg = true
)]
struct Cli {
    /// Run a one-liner
    #[arg(short = 'c', value_name = "code")]
    code: Option<String>,

    /// Run code in awk mode (pattern/action over input)
    #[arg(short = 'a', long = "awk")]
    awk: bool,

    /// Output translated Python and exit
    #[arg(short = 'p', long = "python")]
    python: bool,

    /// Check Snail source formatting (optional paths can scope the search)
    #[arg(long = "format")]
    format: bool,

    /// Update files in place instead of printing unified diffs
    #[arg(long = "write", requires = "format")]
    write: bool,

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

    if cli.format {
        return run_formatting(&cli);
    }

    let mode = if cli.awk {
        CompileMode::Awk
    } else {
        CompileMode::Auto
    };

    let input = if let Some(source) = cli.code {
        CliInput {
            filename: "<cmd>".to_string(),
            source,
            argv: build_argv("-c", cli.args),
            mode,
        }
    } else if let Some((path, script_args)) = cli.args.split_first() {
        let source = read_source(path)?;
        CliInput {
            filename: path.clone(),
            source,
            argv: build_argv(path, script_args.to_vec()),
            mode,
        }
    } else {
        return Err("no input provided; use -c <code> or provide a file".to_string());
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

fn run_formatting(cli: &Cli) -> Result<(), String> {
    if cli.code.is_some() || cli.python || cli.awk {
        return Err("--format cannot be combined with execution flags".to_string());
    }

    let targets: Vec<PathBuf> = if cli.args.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        cli.args.iter().map(PathBuf::from).collect()
    };

    let files = find_snail_files(&targets)?;
    let mut had_changes = false;

    for (idx, path) in files.iter().enumerate() {
        let path_str = path
            .to_str()
            .ok_or_else(|| format!("non-utf8 path: {}", path.display()))?;
        let source = read_source(path_str)?;
        let formatted = format_snail_source(&source);

        if formatted != source {
            had_changes = true;

            if cli.write {
                fs::write(path, &formatted)
                    .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
            }

            let diff = unified_diff(&source, &formatted, path)?;
            if idx > 0 {
                println!();
            }
            print!("{}", diff);
        }
    }

    if had_changes && !cli.write {
        return Err("formatting changes needed".to_string());
    }

    Ok(())
}

fn find_snail_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut queue: VecDeque<PathBuf> = VecDeque::from(paths.to_vec());
    let mut files = Vec::new();

    while let Some(path) = queue.pop_front() {
        let metadata = fs::metadata(&path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;

        if metadata.is_dir() {
            for entry in fs::read_dir(&path)
                .map_err(|err| format!("failed to list {}: {err}", path.display()))?
            {
                let entry =
                    entry.map_err(|err| format!("failed to read {}: {err}", path.display()))?;
                queue.push_back(entry.path());
            }
        } else if metadata.is_file() && is_snail_file(&path) {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

fn is_snail_file(path: &Path) -> bool {
    path.extension().map(|ext| ext == "snail").unwrap_or(false)
}
