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

#[macro_export]
macro_rules! tunable_params {
    ($($name:ident = $val:expr, $min:expr, $max:expr, $step:expr;)*) => {
        #[cfg(feature = "tuning")]
        use std::sync::atomic::Ordering;

        #[cfg(feature = "tuning")]
        pub fn list_params() {
            $(
                println!(
                    "option name {} type spin default {} min {} max {}",
                    stringify!($name),
                    $name(),
                    $min,
                    $max,
                );
            )*
        }

        #[cfg(feature = "tuning")]
        pub fn set_param(name: &str, val: i32) {
            match name {
                $(
                    stringify!($name) => vals::$name.store(val, Ordering::Relaxed),
                )*
                _ => println!("info error unknown option"),
            }
        }

        #[cfg(feature = "tuning")]
        pub fn print_params_ob() {
            $(
                println!(
                    "{}, int, {}.0, {}.0, {}.0, {}, 0.002",
                    stringify!($name),
                    $name(),
                    $min,
                    $max,
                    $step,
                );
            )*
        }

        #[cfg(feature = "tuning")]
        mod vals {
            use std::sync::atomic::AtomicI32;
            $(
            #[allow(non_upper_case_globals)]
            pub static $name: AtomicI32 = AtomicI32::new($val);
            )*
        }

        $(
        #[cfg(feature = "tuning")]
        #[inline]
        pub fn $name() -> i32 {
            vals::$name.load(Ordering::Relaxed)
        }

        #[cfg(not(feature = "tuning"))]
        #[inline]
        pub fn $name() -> i32 {
            $val
        }
        )*
    };
}

pub unsafe fn boxed_and_zeroed<T>() -> Box<T> {
    let layout = std::alloc::Layout::new::<T>();
    let ptr = std::alloc::alloc_zeroed(layout);
    if ptr.is_null() {
        std::alloc::handle_alloc_error(layout);
    }
    Box::from_raw(ptr.cast())
}
