/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! StoreKit `SKPaymentQueue`, `SKPayment`, `SKPaymentTransaction` — offline mock.
//!
//! DRAFT / UNTESTED (2026-07-04): see ../APPLY_STOREKIT_MOCK_README.md.
//!
//! Replaces touchHLE trunk's stub (`canMakePayments`->false, `defaultQueue`->nil).
//! `+defaultQueue` is a real singleton so observers registered on it receive
//! callbacks. `addPayment:` fabricates a `Purchased` transaction and delivers it
//! via `paymentQueue:updatedTransactions:` on the next run-loop turn — the game's
//! observer then grants the item locally. No App Store / Nate server needed.

use super::State;
use crate::frameworks::foundation::ns_string;
use crate::objc::{
    autorelease, id, msg, msg_class, msg_send, nil, objc_classes, release, retain, ClassExports,
    HostObject, NSZonePtr,
};

/// `SKPaymentTransactionState` (from StoreKit headers). The game compares against
/// these compile-time constants, so we only need `Purchased` for the happy path.
#[allow(dead_code)]
const SK_PAYMENT_TRANSACTION_STATE_PURCHASING: i32 = 0;
const SK_PAYMENT_TRANSACTION_STATE_PURCHASED: i32 = 1;
#[allow(dead_code)]
const SK_PAYMENT_TRANSACTION_STATE_FAILED: i32 = 2;
#[allow(dead_code)]
const SK_PAYMENT_TRANSACTION_STATE_RESTORED: i32 = 3;

#[derive(Default)]
struct SKPaymentHostObject {
    /// `NSString*`, owned
    product_identifier: id,
    quantity: i32,
}
impl HostObject for SKPaymentHostObject {}

#[derive(Default)]
struct SKPaymentTransactionHostObject {
    /// `SKPayment*`, owned
    payment: id,
    /// `SKPaymentTransactionState`
    state: i32,
    /// `NSString*`, owned
    transaction_identifier: id,
    /// `NSError*` (nil on success)
    error: id,
    /// `SKPaymentTransaction*` (nil unless restored)
    original_transaction: id,
    /// `NSDate*` (nil is acceptable)
    transaction_date: id,
}
impl HostObject for SKPaymentTransactionHostObject {}

#[derive(Default)]
struct SKPaymentQueueHostObject {
    /// Transaction observers, weak (StoreKit does not retain observers)
    observers: Vec<id>,
    /// In-flight `SKPaymentTransaction*`s, owned until finished
    transactions: Vec<id>,
}
impl HostObject for SKPaymentQueueHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation SKPayment: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(SKPaymentHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)paymentWithProduct:(id)product { // SKProduct*
    let identifier: id = msg![env; product productIdentifier];
    let identifier: id = msg![env; identifier copy]; // +1, owned
    let payment: id = msg_class![env; SKPayment alloc];
    {
        let h = env.objc.borrow_mut::<SKPaymentHostObject>(payment);
        h.product_identifier = identifier;
        h.quantity = 1;
    }
    autorelease(env, payment)
}

- (id)productIdentifier {
    env.objc.borrow::<SKPaymentHostObject>(this).product_identifier
}
- (i32)quantity {
    env.objc.borrow::<SKPaymentHostObject>(this).quantity
}
- (id)applicationUsername {
    nil
}

- (())dealloc {
    let product_identifier = env.objc.borrow::<SKPaymentHostObject>(this).product_identifier;
    release(env, product_identifier);
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

// Some games build the payment through SKMutablePayment. Treat it as SKPayment
// plus a settable quantity.
@implementation SKMutablePayment: SKPayment

- (())setQuantity:(i32)quantity {
    env.objc.borrow_mut::<SKPaymentHostObject>(this).quantity = quantity;
}

@end

@implementation SKPaymentTransaction: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(SKPaymentTransactionHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)payment {
    env.objc.borrow::<SKPaymentTransactionHostObject>(this).payment
}
- (i32)transactionState {
    env.objc.borrow::<SKPaymentTransactionHostObject>(this).state
}
- (id)transactionIdentifier {
    env.objc.borrow::<SKPaymentTransactionHostObject>(this).transaction_identifier
}
- (id)transactionDate {
    env.objc.borrow::<SKPaymentTransactionHostObject>(this).transaction_date
}
- (id)error {
    env.objc.borrow::<SKPaymentTransactionHostObject>(this).error
}
- (id)originalTransaction {
    env.objc.borrow::<SKPaymentTransactionHostObject>(this).original_transaction
}

- (())dealloc {
    let &SKPaymentTransactionHostObject {
        payment,
        transaction_identifier,
        error,
        original_transaction,
        transaction_date,
        ..
    } = env.objc.borrow(this);
    release(env, payment);
    release(env, transaction_identifier);
    release(env, error);
    release(env, original_transaction);
    release(env, transaction_date);
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

@implementation SKPaymentQueue: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(SKPaymentQueueHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (bool)canMakePayments {
    true
}

+ (id)defaultQueue {
    if let Some(queue) = State::get(env).default_queue {
        return queue;
    }
    let queue: id = msg_class![env; SKPaymentQueue alloc];
    let queue: id = msg![env; queue init];
    // Singleton: retained for the lifetime of the process.
    State::get(env).default_queue = Some(queue);
    queue
}

- (())addTransactionObserver:(id)observer {
    let h = env.objc.borrow_mut::<SKPaymentQueueHostObject>(this);
    if !h.observers.contains(&observer) {
        h.observers.push(observer);
    }
}
- (())removeTransactionObserver:(id)observer {
    let h = env.objc.borrow_mut::<SKPaymentQueueHostObject>(this);
    h.observers.retain(|&o| o != observer);
}

- (id)transactions {
    let txns = env.objc.borrow::<SKPaymentQueueHostObject>(this).transactions.clone();
    let array: id = msg_class![env; NSMutableArray array];
    for txn in txns {
        () = msg![env; array addObject:txn];
    }
    array
}

- (())addPayment:(id)payment { // SKPayment*
    retain(env, payment);
    let identifier = ns_string::from_rust_string(env, "touchHLE-mock-transaction".to_string());
    retain(env, identifier);

    let transaction: id = msg_class![env; SKPaymentTransaction alloc]; // +1, owned
    {
        let h = env.objc.borrow_mut::<SKPaymentTransactionHostObject>(transaction);
        h.payment = payment;
        h.state = SK_PAYMENT_TRANSACTION_STATE_PURCHASED;
        h.transaction_identifier = identifier;
    }
    env.objc
        .borrow_mut::<SKPaymentQueueHostObject>(this)
        .transactions
        .push(transaction);

    let sel = env
        .objc
        .register_host_selector("touchHLE_deliverUpdatedTransaction:".to_string(), &mut env.mem);
    () = msg![env; this performSelector:sel withObject:transaction afterDelay:0.0f64];
}

- (())finishTransaction:(id)transaction { // SKPaymentTransaction*
    let sel = env
        .objc
        .register_host_selector("touchHLE_deliverRemovedTransaction:".to_string(), &mut env.mem);
    () = msg![env; this performSelector:sel withObject:transaction afterDelay:0.0f64];
}

- (())restoreCompletedTransactions {
    let sel = env
        .objc
        .register_host_selector("touchHLE_deliverRestoreFinished:".to_string(), &mut env.mem);
    () = msg![env; this performSelector:sel withObject:nil afterDelay:0.0f64];
}

// --- Internal run-loop trampolines (single-arg, so we can use performSelector) ---

- (())touchHLE_deliverUpdatedTransaction:(id)transaction {
    let observers = env.objc.borrow::<SKPaymentQueueHostObject>(this).observers.clone();
    let array: id = msg_class![env; NSMutableArray array];
    () = msg![env; array addObject:transaction];
    let sel = env.objc.register_host_selector(
        "paymentQueue:updatedTransactions:".to_string(),
        &mut env.mem,
    );
    for observer in observers {
        let responds: bool = msg![env; observer respondsToSelector:sel];
        if responds {
            () = msg_send(env, (observer, sel, this, array));
        }
    }
}

- (())touchHLE_deliverRemovedTransaction:(id)transaction {
    let observers = env.objc.borrow::<SKPaymentQueueHostObject>(this).observers.clone();
    let array: id = msg_class![env; NSMutableArray array];
    () = msg![env; array addObject:transaction];
    let sel = env.objc.register_host_selector(
        "paymentQueue:removedTransactions:".to_string(),
        &mut env.mem,
    );
    for observer in observers {
        let responds: bool = msg![env; observer respondsToSelector:sel];
        if responds {
            () = msg_send(env, (observer, sel, this, array));
        }
    }
    // Drop and release the finished transaction.
    let mut released = nil;
    {
        let h = env.objc.borrow_mut::<SKPaymentQueueHostObject>(this);
        if let Some(pos) = h.transactions.iter().position(|&t| t == transaction) {
            released = h.transactions.remove(pos);
        }
    }
    if released != nil {
        release(env, released);
    }
}

- (())touchHLE_deliverRestoreFinished:(id)_arg {
    let observers = env.objc.borrow::<SKPaymentQueueHostObject>(this).observers.clone();
    let sel = env.objc.register_host_selector(
        "paymentQueueRestoreCompletedTransactionsFinished:".to_string(),
        &mut env.mem,
    );
    for observer in observers {
        let responds: bool = msg![env; observer respondsToSelector:sel];
        if responds {
            () = msg_send(env, (observer, sel, this));
        }
    }
}

@end

};
