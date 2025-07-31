set shell := ["powershell.exe", "-c"]

@debug:
    -rm tmp.rs 2>$null
    echo '#![feature(hint_must_use, liballoc_internals, derive_eq, print_internals, structural_match, coverage_attribute, fmt_helpers_for_derive)]' > tmp-utf16.rs
    echo '#![allow(unused_variables,unused_mut,unused_imports)]' >> tmp-utf16.rs
    $(cargo expand --bin test-structured-sql -p test-structured-sql 2>$null) >> tmp-utf16.rs
    Get-Content tmp-utf16.rs | Set-Content -Encoding utf8 tmp.rs
    -cargo +nightly build --bin standalone 2>$null
    rm tmp-utf16.rs

default:
    echo 'Hello, world!'
