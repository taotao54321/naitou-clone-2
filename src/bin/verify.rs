//! verify 用の全棋譜についてクローン、エミュレータ両方の思考ログをとり、一致するか確認する。
//! 一致しなかった思考ログは log/ ディレクトリに出力される。
//!
//! 内部で trace, emu_trace コマンドを利用する。

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long)]
    timelimit: bool,

    #[structopt(parse(from_os_str))]
    path_rom: PathBuf,

    #[structopt(required = true, parse(from_os_str))]
    paths_sfen: Vec<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    const LOG_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/log");

    let opt = Opt::from_args();

    let suite = test_suite(&opt.paths_sfen, opt.timelimit)?;

    for (name, idx, sfen, timelimit) in suite {
        let lineno = idx + 1;
        let filename_base = format!("{}-{:03}", name, lineno);

        let log_lib = trace(&filename_base, &sfen, timelimit)?;
        let log_emu = emu_trace(&opt.path_rom, &filename_base, &sfen, timelimit)?;

        if log_lib == log_emu {
            println!("{}: OK", filename_base);
        } else {
            println!("{}: Failed (logs are saved in {}/", filename_base, LOG_DIR);
            let path_log_lib = format!("{}/{}-lib.log", LOG_DIR, filename_base);
            let path_log_emu = format!("{}/{}-emu.log", LOG_DIR, filename_base);
            std::fs::write(path_log_lib, log_lib)?;
            std::fs::write(path_log_emu, log_emu)?;
        }
    }

    Ok(())
}

/// verify 用の全棋譜を返す。個々の要素は (name, idx, sfen, timelimit)。
fn test_suite<P>(paths: &[P], timelimit: bool) -> anyhow::Result<Vec<(String, usize, String, bool)>>
where
    P: AsRef<Path>,
{
    let mut suite = Vec::<(String, usize, String, bool)>::new();

    for path in paths {
        let path = path.as_ref();

        // *.sfen のみを抽出。
        let name = match path.file_stem().and_then(OsStr::to_str) {
            Some(name) => name,
            None => continue,
        };
        let ext = match path.extension().and_then(OsStr::to_str) {
            Some(ext) => ext,
            None => continue,
        };
        if ext != "sfen" {
            continue;
        }

        // ファイル内の全ての行を sfen 文字列とみなして追加。
        // ただし空行とコメント行(0 個以上の空白と '#' から始まる行)は除く。
        for (idx, line) in std::fs::read_to_string(path)?.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            suite.push((name.to_owned(), idx, line.to_owned(), timelimit));
        }
    }

    Ok(suite)
}

/// trace コマンドによりクローンの思考ログをとる。
fn trace(filename_base: &str, sfen: &str, timelimit: bool) -> anyhow::Result<String> {
    let mut cmd = std::process::Command::new("cargo");

    cmd.args(["run", "--release", "--bin", "trace", "--"]);
    if timelimit {
        cmd.arg("--timelimit");
    }
    cmd.arg(sfen);

    let output = cmd.output()?;

    // 終了ステータスが失敗を示している場合、報告のみ行う。
    if !output.status.success() {
        eprintln!(
            "{}: trace failed:\n{}",
            filename_base,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let res = String::from_utf8(output.stdout)?;

    Ok(res)
}

/// emu_trace コマンドによりエミュレータの思考ログをとる。
fn emu_trace(
    path_rom: &Path,
    filename_base: &str,
    sfen: &str,
    timelimit: bool,
) -> anyhow::Result<String> {
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

    let output = cmd.output()?;

    // 終了ステータスが失敗を示している場合、報告のみ行う。
    if !output.status.success() {
        eprintln!(
            "{}: emu_trace failed:\n{}",
            filename_base,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let res = String::from_utf8(output.stdout)?;

    Ok(res)
}
