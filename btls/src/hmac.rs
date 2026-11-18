use crate::error::ErrorStack;
use crate::foreign_types::ForeignTypeRef;
use crate::hash::MessageDigest;
use crate::{cvt, cvt_p};
use openssl_macros::corresponds;

foreign_type_and_impl_send_sync! {
    type CType = ffi::HMAC_CTX;
    fn drop = ffi::HMAC_CTX_free;

    pub struct HmacCtx;
}

impl HmacCtxRef {
    /// Configures HmacCtx to use `md` as the hash function and `key` as the key.
    ///
    #[corresponds(HMAC_Init_ex)]
    pub fn init(&mut self, key: &[u8], md: &MessageDigest) -> Result<(), ErrorStack> {
        ffi::init();

        unsafe {
            cvt(ffi::HMAC_Init_ex(
                self.as_ptr(),
                key.as_ptr().cast(),
                key.len(),
                md.as_ptr(),
                // ENGINE api is deprecated
                core::ptr::null_mut(),
            ))
        }
    }
}

/// Provides an init-update-finalize API for HMAC.
pub struct Hmac(*mut ffi::HMAC_CTX);

impl Hmac {
    /// Creates a new HMAC object with the given key and hash algorithm.
    pub fn init(key: &[u8], md: &MessageDigest) -> Result<Hmac, ErrorStack> {
        ffi::init();

        let ctx = unsafe { cvt_p(ffi::HMAC_CTX_new())? };
        // Own the allocation before initialization, which can also fail.
        let hmac = Hmac(ctx);
        unsafe {
            cvt(ffi::HMAC_Init_ex(
                hmac.0,
                key.as_ptr().cast(),
                key.len(),
                md.as_ptr(),
                // ENGINE api is deprecated
                core::ptr::null_mut(),
            ))?;
        }

        Ok(hmac)
    }

    /// Updates the HMAC input.
    pub fn update(&mut self, data: &[u8]) -> Result<(), ErrorStack> {
        unsafe { cvt(ffi::HMAC_Update(self.0, data.as_ptr().cast(), data.len())) }
    }

    /// Finalizes the HMAC and returns the output.
    pub fn finalize(self) -> Result<Vec<u8>, ErrorStack> {
        let out_len = unsafe { ffi::HMAC_size(self.0) };
        let mut out = vec![0; out_len];
        unsafe {
            cvt(ffi::HMAC_Final(
                self.0,
                out.as_mut_ptr().cast(),
                // ENGINE api is deprecated
                core::ptr::null_mut(),
            ))?;
        }
        Ok(out)
    }
}

impl Drop for Hmac {
    fn drop(&mut self) {
        unsafe { ffi::HMAC_CTX_free(self.0) }
    }
}

#[cfg(test)]
mod tests {
    use crate::hash;

    use super::*;

    #[test]
    fn allocation_failure_does_not_initialize_null_context() {
        let _failure = ffi::FailureGuard::new(ffi::Failure::Allocation);
        assert!(Hmac::init(b"key", &MessageDigest::sha256()).is_err());
        assert_eq!(ffi::call_counts(), [1, 0, 0]);
    }

    #[test]
    fn initialization_failure_frees_context() {
        let _failure = ffi::FailureGuard::new(ffi::Failure::Initialization);
        assert!(Hmac::init(b"key", &MessageDigest::sha256()).is_err());
        assert_eq!(ffi::call_counts(), [1, 1, 1]);
    }

    fn test<const N: usize>(md: MessageDigest) {
        assert_eq!(N, md.size());
        let key = vec![0; N];
        let message_parts = [
            b"hello".to_vec(),
            b"world!".to_vec(),
            b"".to_vec(),
            vec![0; 23],
            b"fella guy".to_vec(),
        ];
        let message = message_parts.concat();

        let mut hmac = Hmac::init(&key, &md).unwrap();
        for part in &message_parts {
            hmac.update(part).unwrap();
        }
        let res = hmac.finalize().unwrap();
        assert_eq!(res, hash::hmac::<N>(md, &key, &message).unwrap());
    }

    #[test]
    fn test_sha1() {
        test::<20>(MessageDigest::sha1());
    }

    #[test]
    fn test_sha256() {
        test::<32>(MessageDigest::sha256());
    }

    #[test]
    fn test_sha384() {
        test::<48>(MessageDigest::sha384());
    }

    #[test]
    fn test_sha512() {
        test::<64>(MessageDigest::sha512());
    }
}

// Only unit tests intercept these three calls; release builds use crate::ffi.
// Failures are thread-local so other crypto tests still use the native functions.
#[cfg(test)]
#[allow(non_snake_case)]
mod ffi {
    pub(super) use crate::ffi::*;
    use std::cell::Cell;
    use std::ffi::{c_int, c_void};
    use std::ptr;

    #[derive(Clone, Copy, PartialEq)]
    pub(super) enum Failure {
        Allocation,
        Initialization,
    }

    #[derive(Clone, Copy)]
    struct State {
        failure: Option<Failure>,
        calls: [usize; 3],
        allocation: *mut HMAC_CTX,
    }

    thread_local! {
        static STATE: Cell<State> = const { Cell::new(State {
            failure: None,
            calls: [0; 3],
            allocation: ptr::null_mut(),
        }) };
    }

    pub(super) struct FailureGuard(State);

    impl FailureGuard {
        pub(super) fn new(failure: Failure) -> Self {
            STATE.with(|state| {
                let previous = state.get();
                assert!(previous.failure.is_none());
                state.set(State {
                    failure: Some(failure),
                    calls: [0; 3],
                    allocation: ptr::null_mut(),
                });
                Self(previous)
            })
        }
    }

    impl Drop for FailureGuard {
        fn drop(&mut self) {
            let state = STATE.with(|state| state.replace(self.0));
            // Clean up even when a regressed implementation leaks and the test
            // assertion panics. This does not contribute to the call counts.
            if !state.allocation.is_null() {
                unsafe { crate::ffi::HMAC_CTX_free(state.allocation) };
            }
        }
    }

    pub(super) fn call_counts() -> [usize; 3] {
        STATE.with(|state| state.get().calls)
    }

    pub(super) unsafe fn HMAC_CTX_new() -> *mut HMAC_CTX {
        STATE.with(|state| {
            let mut current = state.get();
            if current.failure.is_none() {
                return unsafe { crate::ffi::HMAC_CTX_new() };
            }
            current.calls[0] += 1;
            if current.failure != Some(Failure::Allocation) {
                current.allocation = unsafe { crate::ffi::HMAC_CTX_new() };
                assert!(!current.allocation.is_null());
            }
            state.set(current);
            current.allocation
        })
    }

    pub(super) unsafe fn HMAC_Init_ex(
        ctx: *mut HMAC_CTX,
        key: *const c_void,
        key_len: usize,
        md: *const EVP_MD,
        engine: *mut ENGINE,
    ) -> c_int {
        STATE.with(|state| {
            let mut current = state.get();
            if current.failure.is_none() {
                return unsafe { crate::ffi::HMAC_Init_ex(ctx, key, key_len, md, engine) };
            }
            current.calls[1] += 1;
            state.set(current);
            // If a regression passes a null allocation here, report the extra
            // call through the assertions instead of dereferencing null in C.
            0
        })
    }

    pub(super) unsafe fn HMAC_CTX_free(ctx: *mut HMAC_CTX) {
        STATE.with(|state| {
            let mut current = state.get();
            if current.failure.is_some() {
                current.calls[2] += 1;
                assert_eq!(ctx, current.allocation);
                current.allocation = ptr::null_mut();
                state.set(current);
            }
        });
        unsafe { crate::ffi::HMAC_CTX_free(ctx) };
    }
}
