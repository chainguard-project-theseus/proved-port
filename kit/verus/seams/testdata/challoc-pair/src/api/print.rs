    verified_export_pair! {
        fn malloc_stats_print / je_malloc_stats_print(write_cb: super::WriteCallback, cbopaque: *mut c_void, opts: *const c_char) {
            stats_print_entry(write_cb, cbopaque, opts)
        }
    }
