macro_rules! verified_export_pair {
    ($($(#[$m:meta])* fn $name:ident / $je:ident($($arg:ident: $ty:ty),*) $(-> ($r:ident: $ret:ty))? $(ensures [$($ens:tt)*])? { $($body:tt)* })*) => {
        ::vstd::prelude::verus! {
        $(
            $(#[$m])*
            #[unsafe(no_mangle)]
            #[verifier::assume_unsupported_attributes]
            pub extern "C" fn $name($($arg: $ty),*) $(-> ($r: $ret))?
                $(ensures $($ens)*)?
            {
                proof {
                    crate::sys::grant_syscalls();
                    crate::sys::grant_aborts();
                }
                $($body)*
            }
            $(#[$m])*
            #[unsafe(no_mangle)]
            #[verifier::assume_unsupported_attributes]
            pub extern "C" fn $je($($arg: $ty),*) $(-> ($r: $ret))?
                $(ensures $($ens)*)?
            {
                proof {
                    crate::sys::grant_syscalls();
                    crate::sys::grant_aborts();
                }
                $($body)*
            }
        )*
        } // verus!
    };
}
