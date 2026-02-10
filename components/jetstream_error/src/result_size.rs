// r[impl jetstream.error.v2.result-size]
// r[verify jetstream.error.v2.result-size]
//
// Compile-time assertion that `Error` does not trigger clippy::result_large_err.
// If `Error` grows too large, this module will fail to compile.

#![deny(clippy::result_large_err)]

const _: () = {
    fn _assert_result_not_large() -> crate::Result<()> {
        Ok(())
    }
};
