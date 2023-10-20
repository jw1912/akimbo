pub mod attacks;
pub mod consts;
pub mod frc;
pub mod moves;
pub mod network;
pub mod position;
pub mod search;
pub mod tables;
pub mod thread;

pub const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[macro_export]
macro_rules! bitloop {
    (| $bb:expr, $sq:ident | $func:expr) => {
        while $bb > 0 {
            let $sq = $bb.trailing_zeros() as u8;
            $bb &= $bb - 1;
            $func;
        }
    };
}

#[macro_export]
macro_rules! c_enum {
    ($name:ident : $t:ty, $($n:ident = $v:expr),*) => {
        pub struct $name;
        impl $name {
            $(pub const $n: $t = $v;)*
        }
    }
}

#[macro_export]
macro_rules! init {
    (| $i:ident, $size:literal | $($r:tt)+) => {{
        let mut $i = 0;
        let mut res = [{$($r)+}; $size];
        while $i < $size - 1 {
            $i += 1;
            res[$i] = {$($r)+};
        }
        res
    }}
}

pub fn boxed_and_zeroed<T>() -> Box<T> {
    unsafe {
        let layout = std::alloc::Layout::new::<T>();
        let ptr = std::alloc::alloc_zeroed(layout);
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Box::from_raw(ptr.cast())
    }
}
