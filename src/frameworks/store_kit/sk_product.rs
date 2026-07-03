/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! StoreKit `SKProduct`, `SKProductsRequest`, `SKProductsResponse` — offline mock.
//!
//! DRAFT / UNTESTED (2026-07-04): written against touchHLE trunk conventions but
//! NOT yet built. See ../APPLY_STOREKIT_MOCK_README.md for the 3 residual risk
//! points to check in the first build (get_known_class avoided; NSNumber-as-price;
//! the performSelector async trampoline).
//!
//! Replaces touchHLE trunk's stub. It fabricates a valid `SKProduct` for every
//! requested identifier and delivers `productsRequest:didReceiveResponse:` on the
//! next run-loop turn, so the game sees its cash SKUs as purchasable with no App
//! Store connection.

use crate::frameworks::foundation::{ns_string, NSUInteger};
use crate::objc::{
    autorelease, id, msg, msg_class, msg_send, nil, objc_classes, release, retain, ClassExports,
    HostObject, NSZonePtr,
};
use crate::Environment;

#[derive(Default)]
struct SKProductHostObject {
    /// `NSString*`, owned
    product_identifier: id,
    /// `NSString*`, owned
    localized_title: id,
    /// `NSString*`, owned
    localized_description: id,
    /// `NSNumber*` standing in for `NSDecimalNumber*`, owned
    price: id,
    /// `NSLocale*`, owned
    price_locale: id,
}
impl HostObject for SKProductHostObject {}

#[derive(Default)]
struct SKProductsRequestHostObject {
    /// `NSArray<NSString*>*`, owned
    identifiers: id,
    /// delegate, weak (StoreKit does not retain the request delegate)
    delegate: id,
    /// `SKProductsResponse*`, owned; built at `start`, held until delivered
    response: id,
}
impl HostObject for SKProductsRequestHostObject {}

#[derive(Default)]
struct SKProductsResponseHostObject {
    /// `NSArray<SKProduct*>*`, owned
    products: id,
    /// `NSArray<NSString*>*` (always empty here), owned
    invalid_product_identifiers: id,
}
impl HostObject for SKProductsResponseHostObject {}

/// Build an autoreleased `SKProduct` for one identifier (`identifier` is an
/// `NSString*`). We alloc through the normal `+alloc` path (which routes to our
/// `allocWithZone:` and installs the host object), then fill the ivars.
fn make_product(env: &mut Environment, identifier: id) -> id {
    let product: id = msg_class![env; SKProduct alloc];

    let identifier: id = msg![env; identifier copy]; // +1, owned
    let full = ns_string::to_rust_string(env, identifier).to_string();
    let tail = full.rsplit('.').next().unwrap_or(&full).to_string();
    let title = ns_string::from_rust_string(env, tail); // autoreleased
    retain(env, title);
    let desc = ns_string::from_rust_string(env, full); // autoreleased
    retain(env, desc);
    // NSNumber stands in for NSDecimalNumber (see README caveat).
    let price: id = msg_class![env; NSNumber numberWithDouble:0.99f64]; // autoreleased
    retain(env, price);
    let locale: id = msg_class![env; NSLocale currentLocale]; // autoreleased
    retain(env, locale);

    let h = env.objc.borrow_mut::<SKProductHostObject>(product);
    h.product_identifier = identifier;
    h.localized_title = title;
    h.localized_description = desc;
    h.price = price;
    h.price_locale = locale;

    autorelease(env, product)
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation SKProduct: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(SKProductHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())dealloc {
    let &SKProductHostObject {
        product_identifier,
        localized_title,
        localized_description,
        price,
        price_locale,
    } = env.objc.borrow(this);
    release(env, product_identifier);
    release(env, localized_title);
    release(env, localized_description);
    release(env, price);
    release(env, price_locale);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)productIdentifier {
    env.objc.borrow::<SKProductHostObject>(this).product_identifier
}
- (id)localizedTitle {
    env.objc.borrow::<SKProductHostObject>(this).localized_title
}
- (id)localizedDescription {
    env.objc.borrow::<SKProductHostObject>(this).localized_description
}
- (id)price {
    env.objc.borrow::<SKProductHostObject>(this).price
}
- (id)priceLocale {
    env.objc.borrow::<SKProductHostObject>(this).price_locale
}

@end

@implementation SKProductsResponse: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(SKProductsResponseHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())dealloc {
    let &SKProductsResponseHostObject {
        products,
        invalid_product_identifiers,
    } = env.objc.borrow(this);
    release(env, products);
    release(env, invalid_product_identifiers);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)products {
    env.objc.borrow::<SKProductsResponseHostObject>(this).products
}
- (id)invalidProductIdentifiers {
    env.objc.borrow::<SKProductsResponseHostObject>(this).invalid_product_identifiers
}

@end

@implementation SKProductsRequest: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(SKProductsRequestHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithProductIdentifiers:(id)identifiers { // NSSet<NSString*>*
    let array: id = msg![env; identifiers allObjects]; // NSSet -> NSArray
    let array: id = msg![env; array retain];
    env.objc.borrow_mut::<SKProductsRequestHostObject>(this).identifiers = array;
    this
}

- (id)delegate {
    env.objc.borrow::<SKProductsRequestHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<SKProductsRequestHostObject>(this).delegate = delegate;
}

- (())start {
    let identifiers = env.objc.borrow::<SKProductsRequestHostObject>(this).identifiers;
    let products: id = msg_class![env; NSMutableArray array];
    let count: NSUInteger = msg![env; identifiers count];
    for i in 0..count {
        let ident: id = msg![env; identifiers objectAtIndex:i];
        let product = make_product(env, ident);
        () = msg![env; products addObject:product];
    }
    retain(env, products);
    let invalid: id = msg_class![env; NSArray array];
    retain(env, invalid);

    let response: id = msg_class![env; SKProductsResponse alloc]; // +1, owned
    {
        let h = env.objc.borrow_mut::<SKProductsResponseHostObject>(response);
        h.products = products;
        h.invalid_product_identifiers = invalid;
    }
    env.objc.borrow_mut::<SKProductsRequestHostObject>(this).response = response;

    // Deliver on the next run-loop turn, matching real StoreKit async behaviour.
    let sel = env
        .objc
        .register_host_selector("touchHLE_deliverProductsResponse:".to_string(), &mut env.mem);
    () = msg![env; this performSelector:sel withObject:nil afterDelay:0.0f64];
}

- (())cancel {
    // no-op: the mock delivers synchronously-enough that cancel is a no-op
}

// Internal trampoline invoked by the run loop one turn after `start`.
- (())touchHLE_deliverProductsResponse:(id)_arg {
    let (delegate, response) = {
        let h = env.objc.borrow::<SKProductsRequestHostObject>(this);
        (h.delegate, h.response)
    };
    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "productsRequest:didReceiveResponse:".to_string(),
            &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            () = msg_send(env, (delegate, sel, this, response));
        }
        let fin = env
            .objc
            .register_host_selector("requestDidFinish:".to_string(), &mut env.mem);
        let responds_fin: bool = msg![env; delegate respondsToSelector:fin];
        if responds_fin {
            () = msg_send(env, (delegate, fin, this));
        }
    }
}

- (())dealloc {
    let &SKProductsRequestHostObject {
        identifiers,
        response,
        ..
    } = env.objc.borrow(this);
    release(env, identifiers);
    release(env, response);
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};
