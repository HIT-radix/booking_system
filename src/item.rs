use scrypto::prelude::*;
use scrypto::prelude::rust::cmp;
use crate::reservation::*;

#[derive(Debug, ScryptoSbor)]
struct AvailabilityInterval {
    start_time: i64,
    available: bool,
    price_per_minimum_reservation_period: Option<Decimal>,
}

#[derive(ScryptoSbor)]
pub struct Item {
    id: u64,
    pub owner_id: u64,
    minimum_reservation_period: i64,
    coin: ResourceAddress,
    availability_intervals: KeyValueStore<i64, AvailabilityInterval>,
    availability_interval_list: Vec<i64>,
    reservations: KeyValueStore<u64, Reservation>,
    reservation_list: Vec<u64>,
    min_cancellation_forewarning: i64,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct NewItemEvent {
    item_id: u64,
    owner_id: u64,
    minimum_reservation_period: i64,
    coin: ResourceAddress,
    min_cancellation_forewarning: i64,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct NewAvailabilityIntervalEvent {
    item_id: u64,
    start_time: i64,
    available: bool,
    price_per_minimum_reservation_period: Option<Decimal>,
}

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct UpdateAvailabilityIntervalEvent {
    item_id: u64,
    start_time: i64,
    available: bool,
    price_per_minimum_reservation_period: Option<Decimal>,
}

impl Item {

    pub fn new(
        id: u64,
        owner_id: u64,
        minimum_reservation_period: i64,
        coin: ResourceAddress,
        min_cancellation_forewarning: i64,
    ) -> Item {
        assert!(
            minimum_reservation_period > 0,
            "Negative minimum_reservation_period not allowed",
        );

        assert!(
            min_cancellation_forewarning >= 0,
            "Negative min_cancellation_forewarning not allowed",
        );

        Runtime::emit_event(
            NewItemEvent {
                item_id: id,
                owner_id: owner_id,
                minimum_reservation_period: minimum_reservation_period,
                coin: coin,
                min_cancellation_forewarning: min_cancellation_forewarning,
            }
        );

        Self {
            id: id,
            owner_id: owner_id,
            minimum_reservation_period: minimum_reservation_period,
            coin: coin,
            availability_intervals: KeyValueStore::new(),
            availability_interval_list: vec![],
            reservations: KeyValueStore::new(),
            reservation_list: vec![],
            min_cancellation_forewarning: min_cancellation_forewarning,
        }
    }

    pub fn add_or_modify_availability_interval(
        &mut self,
        start_time: i64,
        available: bool,
        price_per_minimum_reservation_period: Option<Decimal>,
    ) {
        if available {
            assert!(
                price_per_minimum_reservation_period.is_some(),
                "Item must have a price when available",
            );
            assert!(
                price_per_minimum_reservation_period.unwrap() > Decimal::ZERO,
                "Zero price_per_minimum_reservation_period is not allowed",
            );
        }

        let now = Clock::current_time_rounded_to_seconds().seconds_since_unix_epoch;
        match self.availability_interval_list.binary_search(&now) {
            Ok(index) => {
                if index > 0 {
                    self.availability_interval_list.drain(0..index-1);
                }
            }
            Err(index) => {
                if index > 1 {
                    self.availability_interval_list.drain(0..index-2);
                }
            }
        }

        match self.availability_interval_list.binary_search(&start_time) {

            Ok(_) => {
                let mut availability_interval = self.availability_intervals.get_mut(&start_time).unwrap();
                availability_interval.available = available;
                availability_interval.price_per_minimum_reservation_period = price_per_minimum_reservation_period;

                Runtime::emit_event(
                    NewAvailabilityIntervalEvent {
                        item_id: self.id,
                        start_time: start_time,
                        available: available,
                        price_per_minimum_reservation_period: price_per_minimum_reservation_period,
                    }
                );
            }

            Err(_) => {
                let availability_interval = AvailabilityInterval {
                    start_time: start_time,
                    available: available,
                    price_per_minimum_reservation_period: price_per_minimum_reservation_period,
                };
                self.availability_intervals.insert(start_time, availability_interval);
                self.availability_interval_list.push(start_time);
                self.availability_interval_list.sort();

                Runtime::emit_event(
                    UpdateAvailabilityIntervalEvent {
                        item_id: self.id,
                        start_time: start_time,
                        available: available,
                        price_per_minimum_reservation_period: price_per_minimum_reservation_period,
                    }
                );
            }
        }
    }

    pub fn new_reservation(
        &mut self,
        id: u64,
        customer_id: u64,
        start_time: i64,
        end_time: i64,
        mut bucket: Bucket,
        resource_manager: ResourceManager,
    ) -> (Bucket, Bucket) {
        let now = Clock::current_time_rounded_to_seconds().seconds_since_unix_epoch;
        assert!(
            start_time > now,
            "You can only book future dates",
        );

        assert!(
            end_time >= start_time + self.minimum_reservation_period,
            "Reservation length bewlow allowed minimum",
        );

        assert!(
            (end_time - start_time) % self.minimum_reservation_period == 0,
            "Reservation length must be a multiple of {}",
            self.minimum_reservation_period,
        );

        assert!(
            bucket.resource_address() == self.coin,
            "Wrong coin",
        );

        let (mut index, mut availability_interval) = match self.availability_interval_list.binary_search(&start_time) {
            Ok(index) => (
                index,
                self.availability_intervals.get(&start_time).unwrap(),
            ),

            Err(mut index) => {
                assert!(
                    index > 0,
                    "No availability_interval",
                );

                index -= 1;
                let period_start = self.availability_interval_list[index];
                let availability_interval = self.availability_intervals.get(&period_start).unwrap();

                assert!(
                    (start_time - availability_interval.start_time) % self.minimum_reservation_period == 0,
                    "start_time not aligned with availability_intervals",
                );

                (index, availability_interval)
            }
        };

        assert!(
            availability_interval.available,
            "Item not available",
        );

        // Search all availability_intervals for the reservation period and compute total_price
        let mut start_time_in_period = start_time;
        let mut end_time_in_period = match index == self.availability_interval_list.len() - 1 {
            true => end_time,
            false => {
                cmp::min(end_time, self.availability_interval_list[index + 1])
            }
        };

        let mut total_price = availability_interval.price_per_minimum_reservation_period.unwrap() * ((end_time_in_period - start_time_in_period) / self.minimum_reservation_period);

        while end_time_in_period != end_time {
            index += 1;
            availability_interval = self.availability_intervals.get(&end_time_in_period).unwrap();

            assert!(
                availability_interval.available,
                "Item not available",
            );

            start_time_in_period = end_time_in_period;
            end_time_in_period = match index == self.availability_interval_list.len() - 1 {
                true => end_time,
                false => {
                    cmp::min(end_time, self.availability_interval_list[index + 1])
                }
            };

            total_price += availability_interval.price_per_minimum_reservation_period.unwrap() * ((end_time_in_period - start_time_in_period) / self.minimum_reservation_period);
        }

        // Remove past reservations from the list and check that no existing reservation is
        // conflicting with the new one
        self.reservation_list.retain(|reservation_id| {

            let existing_reservation = self.reservations.get(reservation_id).unwrap();

            if existing_reservation.end_time < now ||
                existing_reservation.status == ReservationStatus::CustomerCancelled || 
                existing_reservation.status == ReservationStatus::OwnerCancelled {
                false
            } else {
                assert!(
                    existing_reservation.start_time >= end_time || existing_reservation.end_time <= start_time,
                    "Item not available",
                );
                true
            }
        });

        //TODO: discounts, fees...

        let (reservation, reservation_nft) = Reservation::new(
            id,
            self.id,
            customer_id,
            start_time,
            end_time,
            bucket.take(total_price),
            start_time - self.min_cancellation_forewarning,
        );
        let reservation_bucket = resource_manager.mint_non_fungible(
            &NonFungibleLocalId::integer(id.into()),
            reservation_nft,
        );

        self.reservations.insert(id, reservation);
        self.reservation_list.push(id);

        (reservation_bucket, bucket)
    }

    pub fn get_reservation(
        &mut self,
        reservation_id: u64,
    ) -> KeyValueEntryRefMut<'_, Reservation> {
        self.reservations.get_mut(&reservation_id).expect("Reservation not found")
    }
}
