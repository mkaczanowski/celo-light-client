cfg_if::cfg_if! {
    if #[cfg(feature = "compact-storage")] {
        pub mod compact;
        pub type WasmStorage = compact::DB;
    } else if #[cfg(target_pointer_width = "32")] {
        #[cfg(feature = "simple-storage")]
        pub mod simple;
        pub type WasmStorage = compact::simple;
    }
}
