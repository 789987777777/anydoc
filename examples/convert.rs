//! Convert a document to Markdown: `cargo run --example convert -- <file> [-o out.md] [--bench N]`

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut bench: Option<u32> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                output = args.get(i).map(PathBuf::from);
            }
            "--bench" => {
                i += 1;
                bench = args.get(i).and_then(|v| v.parse().ok());
            }
            other => input = Some(PathBuf::from(other)),
        }
        i += 1;
    }
    let Some(input) = input else {
        eprintln!("usage: convert <file> [-o out.md] [--bench N]");
        return ExitCode::FAILURE;
    };

    if let Some(iters) = bench {
        let mut times = Vec::new();
        let mut out_len = 0;
        for _ in 0..iters.max(1) {
            let start = std::time::Instant::now();
            match anydoc::to_markdown(&input) {
                Ok(md) => out_len = md.len(),
                Err(e) => {
                    eprintln!("error: {e:#}");
                    return ExitCode::FAILURE;
                }
            }
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
        let mean: f64 = times.iter().sum::<f64>() / times.len() as f64;
        let in_len = std::fs::metadata(&input).map(|m| m.len()).unwrap_or(0);
        println!("{}\t{}\t{}\t{:.2}\t{:.2}", input.display(), in_len, out_len, min, mean);
        return ExitCode::SUCCESS;
    }

    match anydoc::to_markdown(&input) {
        Ok(md) => {
            if let Some(out) = output {
                if let Err(e) = std::fs::write(&out, md) {
                    eprintln!("error: failed to write {}: {e}", out.display());
                    return ExitCode::FAILURE;
                }
            } else {
                use std::io::Write;
                let _ = std::io::stdout().write_all(md.as_bytes());
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
