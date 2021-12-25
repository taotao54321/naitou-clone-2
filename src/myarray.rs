//! ユーザー定義型でインデックスアクセスできる配列。

use std::marker::PhantomData;

/// 指定した型でインデックスアクセスできるジェネリック 1 次元配列。
///
/// インデックス型が `usize` に変換可能なことを想定している。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MyArray1<V, K, const N: usize> {
    inner: [V; N],
    _phantom: PhantomData<fn() -> K>,
}

impl<V, K, const N: usize> From<[V; N]> for MyArray1<V, K, N> {
    fn from(inner: [V; N]) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }
}

impl<V, K: Into<usize>, const N: usize> std::ops::Index<K> for MyArray1<V, K, N> {
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        unsafe { self.inner.get_unchecked(index.into()) }
    }
}

impl<V, K: Into<usize>, const N: usize> std::ops::IndexMut<K> for MyArray1<V, K, N> {
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        unsafe { self.inner.get_unchecked_mut(index.into()) }
    }
}

impl<V: Copy + Default, K, const N: usize> Default for MyArray1<V, K, N> {
    fn default() -> Self {
        Self::from([V::default(); N])
    }
}

impl<V, K, const N: usize> std::ops::Deref for MyArray1<V, K, N> {
    type Target = [V; N];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<V, K, const N: usize> std::ops::DerefMut for MyArray1<V, K, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// `MyArray1` の 2 次元版。
#[allow(dead_code)]
pub type MyArray2<V, K1, K2, const N1: usize, const N2: usize> =
    MyArray1<MyArray1<V, K2, N2>, K1, N1>;

/// `MyArray1` の 3 次元版。
#[allow(dead_code)]
pub type MyArray3<V, K1, K2, K3, const N1: usize, const N2: usize, const N3: usize> =
    MyArray1<MyArray2<V, K2, K3, N2, N3>, K1, N1>;

/// `MyArray1` の 4 次元版。
#[allow(dead_code)]
pub type MyArray4<
    V,
    K1,
    K2,
    K3,
    K4,
    const N1: usize,
    const N2: usize,
    const N3: usize,
    const N4: usize,
> = MyArray1<MyArray3<V, K2, K3, K4, N2, N3, N4>, K1, N1>;

/// `MyArray1` の 5 次元版。
#[allow(dead_code)]
pub type MyArray5<
    V,
    K1,
    K2,
    K3,
    K4,
    K5,
    const N1: usize,
    const N2: usize,
    const N3: usize,
    const N4: usize,
    const N5: usize,
> = MyArray1<MyArray4<V, K2, K3, K4, K5, N2, N3, N4, N5>, K1, N1>;
