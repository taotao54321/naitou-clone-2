use std::path::PathBuf;

#[allow(unused_imports)]
use pretty_assertions::{assert_eq, assert_ne};
use walkdir::WalkDir;

use naitou_clone::{sfen_decode, sfen_encode};

const SFEN_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/asset/sfen");

fn get_sfen_paths() -> impl Iterator<Item = PathBuf> {
    WalkDir::new(SFEN_DIR)
        .into_iter()
        .map(|entry| entry.expect("invalid directory entry").into_path())
        .filter(|path| path.extension().map_or(false, |ext| ext == "sfen"))
}

fn get_sfens() -> impl Iterator<Item = String> {
    get_sfen_paths()
        .map(|path| std::fs::read_to_string(path).expect("cannot read sfen file"))
        .flat_map(|body| {
            body.lines()
                .map(|line| line.trim().to_owned())
                .collect::<Vec<_>>()
        })
}

#[test]
fn test_sfen_roundtrip() {
    for sfen in get_sfens() {
        let (side_to_move, board, hands, mvs) = sfen_decode(&sfen).unwrap();
        let sfen_encoded = sfen_encode(side_to_move, &board, &hands, &mvs);
        assert_eq!(sfen, sfen_encoded);
    }
}
