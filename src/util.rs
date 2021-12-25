#![allow(dead_code)]

/// `x < xmin` ならば `xmin = x` と更新する。更新されたかどうかを返す。
pub fn chmin<T: Ord>(xmin: &mut T, x: T) -> bool {
    if x < *xmin {
        *xmin = x;
        true
    } else {
        false
    }
}

/// `xmax < x` ならば `xmax = x` と更新する。更新されたかどうかを返す。
pub fn chmax<T: Ord>(xmax: &mut T, x: T) -> bool {
    if *xmax < x {
        *xmax = x;
        true
    } else {
        false
    }
}
