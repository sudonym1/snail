use std::fs;

fn main() {
    let source =
        fs::read_to_string("benchmarks/large_file.snail").expect("failed to read benchmark file");
    eprintln!(
        "Parsing {} bytes, {} lines",
        source.len(),
        source.lines().count()
    );
    for i in 0..10 {
        let start = std::time::Instant::now();
        let result = snail_parser::parse(&source);
        let elapsed = start.elapsed();
        match result {
            Ok(program) => eprintln!(
                "  iter {i}: {:.3} ms ({} stmts)",
                elapsed.as_secs_f64() * 1000.0,
                program.stmts.len()
            ),
            Err(e) => eprintln!("  iter {i}: parse error: {e}"),
        }
    }
}
