# Booking System

This package implements a generic reservation system for items of any kind.

## Component instantiation

The component can be instantiated with a transaction like this:

    CALL_FUNCTION
        Address("<BLUEPRINT_ADDRESS>")
        BookingSystem
        "new"
        Address("<OWNER_BADGE_ADDRESS>")
    ;

## User

Users can borrow and lend items through the system, this is the transaction for the creation of a new user badge:

    CALL_METHOD
        Address("<COMPONENT_ADDRESS>")
        "new_user"
    ;
    CALL_METHOD
        Address("<ACCOUNT>")
        "deposit_batch"
        Expression("ENTIRE_WORKTOP")
    ;

A `NewUserEvent` is issued; it contains the unique `<USER_BADGE_ID>` assigned to the new user.

## Item

A user can own zero of more items and make them available in the platform by this transaction manifest:

    CALL_METHOD
        Address("<ACCOUNT>")
        "create_proof_of_non_fungibles"
        Address("<USER_BADGE_ADDRESS>")
        Array<NonFungibleLocalId>(NonFungibleLocalId("#<USER_BADGE_ID>#"))
    ;
    POP_FROM_AUTH_ZONE
        Proof("proof")
    ;
    CALL_METHOD
        Address("<COMPONENT_ADDRESS>")
        "new_item"
        Proof("proof")
        <MINIMUM_RESERVATION_PERIOD>i64
        Address("<ACCEPTED_COIN_ADDRESS>")
        <MIN_CANCELLATION_FOREWARNING>i64
    ;

`<MINIMUM_RESERVATION_PERIOD>` is the smallest number of seconds the item can be booked; as an example, 86400i64 if the item can be booked for 1 day.

`<MIN_CANCELLATION_FOREWARNING>` is the smallest forewarning, before the booking start time, the user can cancel a reservation and obtain a full refund. As an example, 604800i64 if the reservation can be cancelled up to one week before the start time.

A `NewItemEvent` is issued; it contains the unique `<ITEM_ID>` assigned to the new item.

The owner of an item can make it available for a time interval at a given cost by this transaction manifest:

    CALL_METHOD
        Address("<ACCOUNT>")
        "create_proof_of_non_fungibles"
        Address("<USER_BADGE_ADDRESS>")
        Array<NonFungibleLocalId>(NonFungibleLocalId("#<USER_BADGE_ID>#"))
    ;
    POP_FROM_AUTH_ZONE
        Proof("proof")
    ;
    CALL_METHOD
        Address("<COMPONENT_ADDRESS>")
        "add_or_modify_availability_interval"
        Proof("proof")
        <ITEM_ID>u64
        <START_TIME>i64
        <AVAILABLE>
        Enum<1u8>(Decimal("<PRICE_PER_MINIMUM_RESERVATION_PERIOD>"))
    ;

`<START_TIME>` is the Unix timestamp of the start of the period. If this timestamp has already been used the the settings replace the previous ones.  
An interval has no explicit end, it just depends on the start of the following interval.

`<AVAILABLE>` is a boolean to set the item available or not for the interval.

`<PRICE_PER_MINIMUM_RESERVATION_PERIOD>` is the cost to book the item for a single `<MINIMUM_RESERVATION_PERIOD>`. In case `<AVAILABLE>` is `false`, this can be None: `Enum<0u8>()`

Depending on the interval being already present or not, a `NewAvailabilityIntervalEvent` or a `UpdateAvailabilityIntervalEvent` event is emitted.

## Reservation

A registered user can book an item using this transaction manifest:

    CALL_METHOD
        Address("<ACCOUNT>")
        "create_proof_of_non_fungibles"
        Address("<USER_BADGE_ADDRESS>")
        Array<NonFungibleLocalId>(NonFungibleLocalId("#<USER_BADGE_ID>#"))
    ;
    POP_FROM_AUTH_ZONE
        Proof("proof")
    ;
    CALL_METHOD
        Address("<ACCOUNT>")
        "withdraw"
        Address("<ACCEPTED_COIN_ADDRESS>")
        Decimal("<PAYMENT_AMOUNT>")
    ;
    TAKE_ALL_FROM_WORKTOP
        Address("<ACCEPTED_COIN_ADDRESS>")
        Bucket("bucket1")
    ;
    CALL_METHOD
        Address("<COMPONENT_ADDRESS>")
        "new_reservation"
        Proof("proof")
        <ITEM_ID>u64
        <START_TIME>i64
        <END_TIME>i64
        Bucket("bucket1")
    ;
    CALL_METHOD
        Address("<ACCOUNT>")
        "deposit_batch"
        Expression("ENTIRE_WORKTOP")
    ;

The component automatically verifies that there are no conflicting reservations for the same item in the same time frame.  

The difference among `<END_TIME>` and `<START_TIME>` must be a multiple of the `<MINIMUM_RESERVATION_PERIOD>`. The `<START_TIME>` must also be an integer number of `<MINIMUM_RESERVATION_PERIOD>` after the `<START_TIME>` of an availability interval. Let's see a simple example: if the `<MINIMUM_RESERVATION_PERIOD>` is one day and the vailability interval starts at 3:00 PM, all reservations must start and end at 3:00 PM.

The customer must pay the full cost at the time of the reservation but the payment is retained by the componet that acts as an escrow.

The transaction returns an NFT containing the reservation details and a `NewReservationEvent` event is issued.

The customer is allowed to cancel a reservation before `<START_TIME>` - `<MIN_CANCELLATION_FOREWARNING>` with this transaction manifest:

    CALL_METHOD
        Address("<ACCOUNT>")
        "withdraw_non_fungibles"
        Address("<RESERVATION_NFT_ADDRESS>")
        Array<NonFungibleLocalId>(NonFungibleLocalId("#<RESERVATION_NFT_ID>#"))
    ;
    TAKE_ALL_FROM_WORKTOP
        Address("<RESERVATION_NFT_ADDRESS>")
        Bucket("bucket1")
    ;
    CALL_METHOD
        Address("<COMPONENT_ADDRESS>")
        "reservation_cancellation_by_customer"
        Bucket("bucket1")
    ;
    CALL_METHOD
        Address("<ACCOUNT>")
        "deposit_batch"
        Expression("ENTIRE_WORKTOP")
    ;

The resevation NFT is burned, a `ReservationCustomerCancellationEvent` is issued and the customer immediately receives a full refund.

The customer can no longer cancel a reservation after `<START_TIME>` - `<MIN_CANCELLATION_FOREWARNING>`.

The owner of an item can cancel a reservation at any time; this is the transaction manifest to use:

    CALL_METHOD
        Address("<ACCOUNT>")
        "create_proof_of_non_fungibles"
        Address("<USER_BADGE_ADDRESS>")
        Array<NonFungibleLocalId>(NonFungibleLocalId("#<USER_BADGE_ID>#"))
    ;
    POP_FROM_AUTH_ZONE
        Proof("proof")
    ;
    CALL_METHOD
        Address("<COMPONENT_ADDRESS>")
        "reservation_cancellation_by_owner"
        <ITEM_ID>u64
        <RESERVATION_ID>u64
    ;

A `ReservationOwnerCancellationEvent` is issued and a full refund is accrued to the custemer that can claim it later.

## Refund

A customer can claim its accrued refund by this transaction:

    CALL_METHOD
        Address("<ACCOUNT>")
        "withdraw_non_fungibles"
        Address("<RESERVATION_NFT_ADDRESS>")
        Array<NonFungibleLocalId>(NonFungibleLocalId("#<RESERVATION_NFT_ID>#"))
    ;
    TAKE_ALL_FROM_WORKTOP
        Address("<RESERVATION_NFT_ADDRESS>")
        Bucket("bucket1")
    ;
    CALL_METHOD
        Address("<COMPONENT_ADDRESS>")
        "get_refund"
    ;
    CALL_METHOD
        Address("<ACCOUNT>")
        "deposit_batch"
        Expression("ENTIRE_WORKTOP")
    ;

A `ReservationRefundEvent` event is issued.

## Payment

By default an item owner can get the payment for a reservation as soon as the reservation's `<END_TIME>` has passed. The component owner can set an additional delay for the payments through this transaction:

    CALL_METHOD
        Address("<ACCOUNT>")
        "create_proof_of_amount"
        Address("<OWNER_BADGE_ADDRESS>")
        Decimal("1")
    ;
        CALL_METHOD
        Address("<COMPONENT_ADDRESS>")
        "set_payment_delay"
        <PAYMENT_DELAY>i64
    ;

This is the transaction to get the payment:

    CALL_METHOD
        Address("<ACCOUNT>")
        "create_proof_of_non_fungibles"
        Address("<USER_BADGE_ADDRESS>")
        Array<NonFungibleLocalId>(NonFungibleLocalId("#<USER_BADGE_ID>#"))
    ;
    POP_FROM_AUTH_ZONE
        Proof("proof")
    ;
    CALL_METHOD
        Address("<COMPONENT_ADDRESS>")
        "get_payment"
        <ITEM_ID>u64
        <RESERVATION_ID>u64
    ;
    CALL_METHOD
        Address("<ACCOUNT>")
        "deposit_batch"
        Expression("ENTIRE_WORKTOP")
    ;

A `ReservationGetPaymentEvent` event is issued.

## Dispute

If a customer is not satisfied with his reservation he can dispute it so the owner is no longer allowed to withdraw the payment until the dispute is solved in a way or another. This is the transaction to open a dispute:

    CALL_METHOD
        Address("<ACCOUNT>")
        "create_proof_of_non_fungibles"
        Address("<RESERVATION_NFT_ADDRESS>")
        Array<NonFungibleLocalId>(NonFungibleLocalId("#<RESERVATION_NFT_ID>#"))
    ;
    POP_FROM_AUTH_ZONE
        Proof("proof")
    ;
    CALL_METHOD
        Address("<COMPONENT_ADDRESS>")
        "start_dispute"
    ;

A `ReservationDisputeEvent` event is issued.

The owner of an item can offer a partial refund for a disputed reservation:

    CALL_METHOD
        Address("<ACCOUNT>")
        "create_proof_of_non_fungibles"
        Address("<USER_BADGE_ADDRESS>")
        Array<NonFungibleLocalId>(NonFungibleLocalId("#<USER_BADGE_ID>#"))
    ;
    POP_FROM_AUTH_ZONE
        Proof("proof")
    ;
    CALL_METHOD
        Address("<COMPONENT_ADDRESS>")
        "offer_partial_refund"
        <ITEM_ID>u64
        <RESERVATION_ID>u64
        Decimal("<REFUND_AMOUNT>")
    ;

A `ReservationRefundOfferEvent` event is issued.

The item owner can replace an offer with a new one using the same transaction manifest.

If the customer claims the offered refund the dispute is terminated and the owner can withdraw his partial payment too.

## Arbitrator

Another way to terminate a dispute is through the arbitrators' vote.

The owner of the component can mint arbitrator badges; the received token is not transferrable and recallable so the component owner keeps full control of who the arbitrators are.

    CALL_METHOD
        Address("<ACCOUNT>")
        "create_proof_of_amount"
        Address("<OWNER_BADGE_ADDRESS>")
        Decimal("1")
    ;
    CALL_METHOD
        Address("<COMPONENT_ADDRESS>")
        "get_arbitrator_badge"
    ;
    CALL_METHOD
        Address("<ARBITRATOR_ACCOUNT>")
        "try_deposit_batch_or_abort"
        Expression("ENTIRE_WORKTOP")
    ;

The component owner can also decide how many arbitrators' votes are needed to terminate a dispute.

    CALL_METHOD
        Address("<ACCOUNT>")
        "create_proof_of_amount"
        Address("<OWNER_BADGE_ADDRESS>")
        Decimal("1")
    ;
    CALL_METHOD
        Address("<COMPONENT_ADDRESS>")
        "set_min_arbitrators"
        <MIN_ARBITRATORS>u16
    ;

An arbitator can vote on the refund percentage to the customer for a disputed reservation:

    CALL_METHOD
        Address("<ACCOUNT>")
        "create_proof_of_non_fungibles"
        Address("<ARBITRATOR_BADGE_ADDRESS>")
        Array<NonFungibleLocalId>(NonFungibleLocalId("#<ARBITRATOR_BADGE_ID>#"))
    ;
    POP_FROM_AUTH_ZONE
        Proof("proof")
    ;
    CALL_METHOD
        Address("<COMPONENT_ADDRESS>")
        "dispute_vote"
        Proof("proof")
        <ITEM_ID>u64
        <RESERVATION_ID>u64
        Decimal("<REFUND_PERCENTAGE>")
    ;

A `DisputeVoteEvent` is issued; if a sufficient number of arbitrators has voted the dispute is closed and a `DisputeVoteTerminatedEvent` is issued.
 
An arbitrator is also allowed to change his mind and modify his vote on a dispute before it terminates.

The actual refund and payment amounts depends on the average of the `<REFUND_PERCENTAGE>` in the arbitrators' votes.
