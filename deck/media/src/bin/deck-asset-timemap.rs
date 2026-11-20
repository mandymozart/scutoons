//! `deck-asset-timemap <source> <derivative>`: the derivative's time map against its source, as JSON on stdout
//! (`{"offset":0.5,"fps":24}`: the source's time at the derivative's 0, the derivative's frames per second). The asset
//! pipeline (server AssetJobs) calls it after each derivative. Failure: a reason on stderr, exit code 1 (2: usage).
use deck_media::{timemap, Clip};
use std::process::ExitCode;

fn open(path: &str) -> Result<Clip, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    Clip::open(bytes.into()).map_err(|e| format!("{path}: {e}"))
}

fn measure(source: &str, derivative: &str) -> Result<String, String> {
    let map = timemap::measure(&open(source)?, &open(derivative)?).map_err(|e| format!("{derivative}: {e}"))?;
    Ok(format!("{{\"offset\":{},\"fps\":{}}}", map.offset, map.fps))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [source, derivative] = args.as_slice() else {
        eprintln!("usage: deck-asset-timemap <source> <derivative>");
        return ExitCode::from(2);
    };
    match measure(source, derivative) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("{why}");
            ExitCode::FAILURE
        }
    }
}
