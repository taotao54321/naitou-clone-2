//! FCEUX へのインターフェース。

use std::path::Path;

pub use fceux::RegP;

/// 単一のコントローラーのボタン入力(複数のボタンの組み合わせ)。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Buttons(u8);

pub const BUTTONS_A: Buttons = Buttons(1 << 0);
pub const BUTTONS_B: Buttons = Buttons(1 << 1);
pub const BUTTONS_S: Buttons = Buttons(1 << 2);
pub const BUTTONS_T: Buttons = Buttons(1 << 3);
pub const BUTTONS_U: Buttons = Buttons(1 << 4);
pub const BUTTONS_D: Buttons = Buttons(1 << 5);
pub const BUTTONS_L: Buttons = Buttons(1 << 6);
pub const BUTTONS_R: Buttons = Buttons(1 << 7);

pub const BUTTONS_UL: Buttons = BUTTONS_U.or(BUTTONS_L);
pub const BUTTONS_UR: Buttons = BUTTONS_U.or(BUTTONS_R);
pub const BUTTONS_DL: Buttons = BUTTONS_D.or(BUTTONS_L);
pub const BUTTONS_DR: Buttons = BUTTONS_D.or(BUTTONS_R);

impl Buttons {
    /// 無入力を返す。
    pub const fn empty() -> Self {
        Self(0)
    }

    /// 無入力かどうかを返す。
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// 2 つの入力の AND を返す。
    pub const fn and(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// 2 つの入力の OR を返す。
    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// 内部値を返す。
    const fn inner(self) -> u8 {
        self.0
    }
}

impl std::ops::BitAnd<Self> for Buttons {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self {
        self.and(rhs)
    }
}

impl std::ops::BitAndAssign<Self> for Buttons {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl std::ops::BitOr<Self> for Buttons {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        self.or(rhs)
    }
}

impl std::ops::BitOrAssign<Self> for Buttons {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl std::fmt::Display for Buttons {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use std::fmt::Write as _;

        const TABLE: [(Buttons, char); 8] = [
            (BUTTONS_R, 'R'),
            (BUTTONS_L, 'L'),
            (BUTTONS_D, 'D'),
            (BUTTONS_U, 'U'),
            (BUTTONS_T, 'T'),
            (BUTTONS_S, 'S'),
            (BUTTONS_B, 'B'),
            (BUTTONS_A, 'A'),
        ];

        for (button, c) in TABLE {
            if (*self & button).is_empty() {
                f.write_char('.')?;
            } else {
                f.write_char(c)?;
            }
        }

        Ok(())
    }
}

/// エミュレータ (FCEUX) を初期化する。**プログラム起動直後に必ずこれを呼ぶこと**。
pub fn init(path_rom: impl AsRef<Path>) -> anyhow::Result<()> {
    fceux::init(path_rom)?;

    Ok(())
}

/// NES のカラーコード `idx` に対応する `(r, g, b)` 値を返す。
pub fn nes_color(idx: u8) -> (u8, u8, u8) {
    fceux::video_get_palette(idx)
}

/// エミュレータの CPU P レジスタを読み取る。
pub fn reg_p() -> RegP {
    fceux::reg_p()
}

/// 論理アドレスを指定してエミュレータのメモリを読み取る。
pub fn memory_read(addr: u16) -> u8 {
    fceux::mem_read(addr, fceux::MemoryDomain::Cpu)
}

/// 入力およびビデオ/サウンド処理関数を与えた上でエミュレータを 1 フレーム動かす。
pub fn run_frame<VideoSoundF>(buttons: Buttons, f_video_sound: VideoSoundF)
where
    VideoSoundF: FnOnce(&[u8], &[i32]),
{
    run_frame_hooked(buttons, f_video_sound, &|_| {});
}

/// 入力、ビデオ/サウンド処理関数、アドレス実行フック関数を与えた上でエミュレータを 1 フレーム動かす。
pub fn run_frame_hooked<VideoSoundF>(
    buttons: Buttons,
    f_video_sound: VideoSoundF,
    f_hook: &dyn FnMut(u16),
) where
    VideoSoundF: FnOnce(&[u8], &[i32]),
{
    fceux::run_frame(buttons.inner(), 0, f_video_sound, f_hook);
}

/// 入力を与えた上でエミュレータを 1 フレーム動かす。
pub fn run_frame_headless(buttons: Buttons) {
    run_frame_hooked_headless(buttons, &|_| {});
}

/// 入力およびアドレス実行フック関数を与えた上でエミュレータを 1 フレーム動かす。
pub fn run_frame_hooked_headless(buttons: Buttons, f_hook: &dyn FnMut(u16)) {
    fceux::run_frame(buttons.inner(), 0, |_, _| {}, f_hook);
}

/// エミュレータの状態のスナップショット。ステートセーブ/ロード用。
pub type Snapshot = fceux::Snapshot;

/// `Snapshot` オブジェクトを作成する。これだけではステートセーブは行われない。
pub fn snapshot_create() -> Snapshot {
    fceux::snapshot_create()
}

/// ステートセーブする。
pub fn snapshot_load(snap: &Snapshot) {
    fceux::snapshot_load(snap).expect("snapshot_load() failed");
}

/// ステートロードする。
pub fn snapshot_save(snap: &Snapshot) {
    fceux::snapshot_save(snap).expect("snapshot_save() failed");
}
