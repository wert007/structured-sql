// use std::mem::MaybeUninit;

// #[macro_export]
// macro_rules! concat_sql_columns {
//     ($slice:expr) => {{
//         const __ARGS_81608BFNA5: &[&[silo::SqlColumn]] = $slice;
//         {
//             const LEN: usize = silo::derive::__concat_sum_lengths(__ARGS_81608BFNA5);

//             const CONC: [silo::SqlColumn; LEN] = silo::derive::__concat_slices(__ARGS_81608BFNA5);

//             CONC
//         }
//     }};
// }

// pub const fn __concat_sum_lengths<T>(slice: &[&[T]]) -> usize {
//     let mut sum = 0usize;
//     let mut i = 0;
//     while i < slice.len() {
//         sum += slice[i].len();
//         i += 1;
//     }
//     sum
// }

// pub const fn __concat_slices<T, const N: usize>(slices: &[&[T]]) -> [T; N] {
//     let mut out = [const { MaybeUninit::uninit() }; N];
//     let mut out_i = 0usize;

//     let mut si = 0;
//     while si < slices.len() {
//         let slice = slices[si];
//         let mut i = 0;
//         while i < slice.len() {
//             out[out_i] = MaybeUninit::new(slice[i]);
//             out_i += 1;
//             i += 1;
//         }
//         si += 1;
//     }

//     unsafe { std::mem::transmute_copy(&out) }
// }

// fn empty_array<T>() -> [T; 0]
// where
//     T: Copy,
// {
//     []
// }
