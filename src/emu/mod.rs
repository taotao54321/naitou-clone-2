//! 原作をエミュレータ (FCEUX) 上で操作するモジュール。
//! 思考ログの verify に使う。
//!
//! **プログラム起動直後に必ず `init()` で初期化すること**。

pub mod addrs;
pub mod apsp;
mod backend;
mod naitou;

pub use self::backend::*;
pub use self::naitou::*;
