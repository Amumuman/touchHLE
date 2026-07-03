/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! StoreKit — offline in-app-purchase mock.
//!
//! DRAFT / UNTESTED (2026-07-04). Replaces touchHLE trunk's stub so TalesWeaver
//! Lucian (iOS) can complete a cash purchase entirely offline. See
//! APPLY_STOREKIT_MOCK_README.md.

mod sk_payment_queue;
mod sk_product;

use crate::objc::id;

#[derive(Default)]
pub struct State {
    /// `SKPaymentQueue*` singleton returned by `+[SKPaymentQueue defaultQueue]`.
    default_queue: Option<id>,
}
impl State {
    fn get(env: &mut crate::Environment) -> &mut State {
        &mut env.framework_state.store_kit
    }
}

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/System/Library/Frameworks/StoreKit.framework/StoreKit",
    aliases: &[],
    class_exports: &[sk_payment_queue::CLASSES, sk_product::CLASSES],
    constant_exports: &[],
    function_exports: &[],
};
