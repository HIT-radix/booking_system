use scrypto::prelude::*;

#[derive(Debug, ScryptoSbor, PartialEq, Clone, Copy)]
pub enum ReservationStatus {
    Booked,
    CustomerCancelled,
    OwnerCancelled,
    Disputing,
    DisputeTerminated,
    Completed,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct NewReservationEvent {
    reservation_id: u64,
    item_id: u64,
    customer_id: u64,
    start_time: i64,
    end_time: i64,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct ReservationCustomerCancellationEvent {
    reservation_id: u64,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct ReservationOwnerCancellationEvent {
    reservation_id: u64,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct ReservationRefundEvent {
    reservation_id: u64,
    old_status: ReservationStatus,
    new_status: ReservationStatus,
    refund_amount: Decimal,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct ReservationDisputeEvent {
    reservation_id: u64,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct ReservationRefundOfferEvent {
    reservation_id: u64,
    refund_amount: Decimal,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct ReservationGetPaymentEvent {
    reservation_id: u64,
    old_status: ReservationStatus,
    new_status: ReservationStatus,
    payment_amount: Decimal
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct DisputeVoteEvent {
    reservation_id: u64,
    arbitrator_id: u64,
    number_of_voters: usize,
    min_arbitrators: u16,
    dispute_votes_sum: Decimal,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct DisputeVoteTerminatedEvent {
    reservation_id: u64,
    refund_amount: Decimal,
    to_owner: Decimal,
}

#[derive(Debug, ScryptoSbor, NonFungibleData)]
pub struct Reservation {
    id: u64,
    customer_id: u64,
    pub start_time: i64,
    pub end_time: i64,
    vault: Vault,
    pub status: ReservationStatus,
    refund_amount: Decimal,
    to_owner: Decimal,
    dispute_votes: BTreeMap<u64, Decimal>,
    dispute_votes_sum: Decimal,
}

#[derive(Debug, ScryptoSbor, NonFungibleData)]
pub struct ReservationNFT {
    pub id: u64,
    pub item_id: u64,
    pub start_time: Instant,
    pub end_time: Instant,
    pub status: ReservationStatus,
    pub max_cancellation_time: Instant,
}

impl Reservation {

    pub fn new(
        id: u64,
        item_id: u64,
        customer_id: u64,
        start_time: i64,
        end_time: i64,
        bucket: Bucket,
        max_cancellation_time: i64,
    ) -> (Reservation, ReservationNFT) {

        Runtime::emit_event(
            NewReservationEvent {
                reservation_id: id,
                item_id: item_id,
                customer_id: customer_id,
                start_time: start_time,
                end_time: end_time,
            }
        );

        (
            Self {
                id: id,
                to_owner: bucket.amount(),
                customer_id: customer_id,
                start_time: start_time,
                end_time: end_time,
                vault: Vault::with_bucket(bucket),
                status: ReservationStatus::Booked,
                refund_amount: Decimal::ZERO,
                dispute_votes_sum: Decimal::ZERO,
                dispute_votes: BTreeMap::new(),
            },
            ReservationNFT {
                id: id,
                item_id: item_id,
                start_time: Instant {
                    seconds_since_unix_epoch: start_time
                },
                end_time: Instant {
                    seconds_since_unix_epoch: end_time
                },
                status: ReservationStatus::Booked,
                max_cancellation_time: Instant {
                    seconds_since_unix_epoch: max_cancellation_time
                }
            }
        )
    }

    pub fn cancellation_by_customer(
        &mut self
    ) -> Bucket {
        assert!(
            self.status == ReservationStatus::Booked,
            "Wrong status",
        );
        self.status = ReservationStatus::CustomerCancelled;

        Runtime::emit_event(
            ReservationCustomerCancellationEvent {
                reservation_id: self.id,
            }
        );

        self.vault.take_all()
    }

    pub fn cancellation_by_owner(
        &mut self,
    ) {
        assert!(
            self.status == ReservationStatus::Booked ||
            self.status == ReservationStatus::Disputing,
            "Wrong status",
        );

        Runtime::emit_event(
            ReservationOwnerCancellationEvent {
                reservation_id: self.id,
            }
        );

        self.status = ReservationStatus::OwnerCancelled;
    }

    pub fn get_refund(
        &mut self,
    ) -> Bucket {
        let old_status = self.status;

        let refund = match self.status {
            ReservationStatus::OwnerCancelled => self.vault.take_all(),

            ReservationStatus::Disputing => {
                self.status = ReservationStatus::DisputeTerminated;
                let refund_amount = self.refund_amount;
                self.refund_amount = Decimal::ZERO;
                self.vault.take(refund_amount)
            },

            ReservationStatus::DisputeTerminated => {
                let refund_amount = self.refund_amount;
                self.refund_amount = Decimal::ZERO;
                self.vault.take(refund_amount)
            }

            _ => Runtime::panic("No refund available".to_string()),
        };

        Runtime::emit_event(
            ReservationRefundEvent {
                reservation_id: self.id,
                old_status: old_status,
                new_status: self.status,
                refund_amount: refund.amount(),
            }
        );

        refund
    }

    pub fn start_dispute(
        &mut self,
    ) {
        assert!(
            self.status == ReservationStatus::Booked,
            "Wrong status",
        );

        self.status = ReservationStatus::Disputing;

        Runtime::emit_event(
            ReservationDisputeEvent {
                reservation_id: self.id,
            }
        );
    }

    pub fn offer_partial_refund(
        &mut self,
        refund_amount: Decimal,
    ) {
        assert!(
            self.status == ReservationStatus::Disputing,
            "Wrong status",
        );
        assert!(
            refund_amount < Decimal::ZERO,
            "Negative refund not possible",
        );
        assert!(
            refund_amount <= self.vault.amount(),
            "Refund bigger than payment",
        );

        self.refund_amount = refund_amount;
        self.to_owner = self.vault.amount() - refund_amount;

        Runtime::emit_event(
            ReservationRefundOfferEvent {
                reservation_id: self.id,
                refund_amount: refund_amount,
            }
        );
    }

    pub fn get_payment(
        &mut self,
        payment_delay: i64,
    ) -> Bucket {
        let old_status = self.status;

        let payment = match self.status {
            ReservationStatus::Booked => {
                let now = Clock::current_time_rounded_to_seconds().seconds_since_unix_epoch;
                assert!(
                    now >= self.end_time + payment_delay,
                    "You can't get the payment now",
                );
                self.status = ReservationStatus::Completed;
                self.vault.take_all()
            },

            ReservationStatus::DisputeTerminated => {
                let payment_amount = self.to_owner;
                self.to_owner = Decimal::ZERO;
                self.vault.take(payment_amount)
            },

            _ => Runtime::panic("Wrong status".to_string())
        };

        Runtime::emit_event(
            ReservationGetPaymentEvent {
                reservation_id: self.id,
                old_status: old_status,
                new_status: self.status,
                payment_amount: payment.amount(),
            }
        );

        payment
    }

    pub fn dispute_vote(
        &mut self,
        arbitrator_id: u64,
        refund_percentage: Decimal,
        min_arbitrators: u16,
    ) -> bool {
        assert!(
            self.status == ReservationStatus::Disputing,
            "Wrong status",
        );

        self.dispute_votes_sum += refund_percentage;
        let old_vote = self.dispute_votes.insert(arbitrator_id, refund_percentage);
        if old_vote.is_some() {
            self.dispute_votes_sum -= old_vote.unwrap();
        }

        let number_of_voters = self.dispute_votes.len();

        Runtime::emit_event(
            DisputeVoteEvent {
                reservation_id: self.id,
                arbitrator_id: arbitrator_id,
                number_of_voters: number_of_voters,
                min_arbitrators: min_arbitrators,
                dispute_votes_sum: self.dispute_votes_sum,
            }
        );

        if number_of_voters >= min_arbitrators.into() {
            self.status = ReservationStatus::DisputeTerminated;
            self.refund_amount = (self.dispute_votes_sum / number_of_voters) * self.vault.amount();
            self.to_owner = self.vault.amount() - self.refund_amount;

            Runtime::emit_event(
                DisputeVoteTerminatedEvent {
                    reservation_id: self.id,
                    refund_amount: self.refund_amount,
                    to_owner: self.to_owner,
                }
            );

            return true;
        };

        false
    }
}
