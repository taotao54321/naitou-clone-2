//! verify 用の全棋譜についてクローン、エミュレータ両方の思考ログをとり、一致するか確認する。
//! 一致しなかった思考ログは log/ ディレクトリに出力される。
//!
//! 内部で trace, emu_trace コマンドを利用する。

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(parse(from_os_str))]
    path_rom: PathBuf,
}

fn main() -> anyhow::Result<()> {
    const LOG_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/log");

    let opt = Opt::from_args();

    let suite = test_suite()?;

    for (stem, sfen, timelimit) in suite {
        let log_lib = trace(&sfen, timelimit)?;
        let log_emu = emu_trace(&opt.path_rom, &sfen, timelimit)?;

        if log_lib == log_emu {
            println!("{}: OK", stem);
        } else {
            println!("{}: Failed (logs are saved in {}/", stem, LOG_DIR);
            let path_log_lib = format!("{}/{}-lib.log", LOG_DIR, stem);
            let path_log_emu = format!("{}/{}-emu.log", LOG_DIR, stem);
            std::fs::write(path_log_lib, log_lib)?;
            std::fs::write(path_log_emu, log_emu)?;
        }
    }

    Ok(())
}

/// verify 用の全棋譜を返す。個々の要素は (stem, sfen, timelimit)。
///
/// tests/asset/verify/{notimelimit,timelimit}/ 以下の *.sfen ファイルが対象。
fn test_suite() -> anyhow::Result<Vec<(String, String, bool)>> {
    const VERIFY_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/asset/verify");

    let paths_notimelimit = paths_in_directory(format!("{}/notimelimit", VERIFY_DIR))?;
    let paths_timelimit = paths_in_directory(format!("{}/timelimit", VERIFY_DIR))?;

    let it = paths_notimelimit
        .into_iter()
        .map(|path| (path, false))
        .chain(paths_timelimit.into_iter().map(|path| (path, true)));

    let mut suite = Vec::<(String, String, bool)>::new();

    for (path, timelimit) in it {
        // *.sfen のみを抽出。
        let stem = match path.file_stem().and_then(OsStr::to_str) {
            Some(stem) => stem,
            None => continue,
        };
        let ext = match path.extension().and_then(OsStr::to_str) {
            Some(ext) => ext,
            None => continue,
        };
        if ext != "sfen" {
            continue;
        }

        let sfen = std::fs::read_to_string(&path)?;
        suite.push((stem.to_owned(), sfen, timelimit));
    }

    Ok(suite)
}

/// 指定したディレクトリ直下の全てのパスを返す。
fn paths_in_directory(dir: impl AsRef<Path>) -> anyhow::Result<Vec<PathBuf>> {
    let paths = std::fs::read_dir(dir)?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(paths)
}

/// trace コマンドによりクローンの思考ログをとる。
fn trace(sfen: &str, timelimit: bool) -> anyhow::Result<String> {
    let mut cmd = std::process::Command::new("cargo");

    cmd.args(["run", "--release", "--bin", "trace", "--"]);
    if timelimit {
        cmd.arg("--timelimit");
    }
    cmd.arg(sfen);

    let res = String::from_utf8(cmd.output()?.stdout)?;

    Ok(res)
}

/// emu_trace コマンドによりエミュレータの思考ログをとる。
fn emu_trace(path_rom: &Path, sfen: &str, timelimit: bool) -> anyhow::Result<String> {
    let mut cmd = std::process::Command::new("cargo");

    cmd.args([
        "run",
        "--features",
        "emu,sdl",
        "--release",
        "--bin",
        "emu_trace",
        "--",
    ]);
    if timelimit {
        cmd.arg("--timelimit");
    }
    cmd.args([path_rom.to_str().unwrap(), sfen]);

    let res = String::from_utf8(cmd.output()?.stdout)?;

    Ok(res)
}
