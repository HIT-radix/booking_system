use scrypto::prelude::*;
use crate::user::*;
use crate::item::*;
use crate::reservation::*;
use crate::arbitrator::*;

#[blueprint]
#[events(
    NewUserEvent,
    NewItemEvent,
    NewAvailabilityIntervalEvent,
    UpdateAvailabilityIntervalEvent,
    NewReservationEvent,
    ReservationCustomerCancellationEvent,
    ReservationOwnerCancellationEvent,
    ReservationRefundEvent,
    ReservationDisputeEvent,
    ReservationRefundOfferEvent,
    DisputeVoteEvent,
    DisputeVoteTerminatedEvent,
    NewArbitratorEvent,
)]
mod booking_system {

    enable_method_auth! {
        roles {
            arbitrator => updatable_by: [OWNER];
        },
        methods {
            set_payment_delay => restrict_to: [OWNER];
            get_arbitrator_badge => restrict_to: [OWNER];
            set_min_arbitrators => restrict_to: [OWNER];

            new_user => PUBLIC;

            new_item => PUBLIC;
            add_or_modify_availability_interval => PUBLIC;
            reservation_cancellation_by_owner => PUBLIC;
            offer_partial_refund => PUBLIC;
            get_payment => PUBLIC;

            new_reservation => PUBLIC;
            reservation_cancellation_by_customer => PUBLIC;
            get_refund => PUBLIC;
            start_dispute => PUBLIC;

            dispute_vote => PUBLIC;
        }
    }

    struct BookingSystem {
        last_user_id: u64,
        users_resource_manager: ResourceManager,

        last_item_id: u64,
        items: KeyValueStore<u64, Item>,

        last_reservation_id: u64,
        reservations_resource_manager: ResourceManager,

        last_arbitrator_id: u64,
        arbitrators_resource_manager: ResourceManager,
        min_arbitrators: u16,

        payment_delay: i64,
    }

    impl BookingSystem {

        pub fn new(
            owner_badge_address: ResourceAddress,
        ) -> Global<BookingSystem> {

            let (address_reservation, component_address) =
                Runtime::allocate_component_address(BookingSystem::blueprint_id());

            let users_resource_manager = ResourceBuilder::new_integer_non_fungible::<User>(
                OwnerRole::Updatable(rule!(require(owner_badge_address)))
            )
            .metadata(metadata!(
                roles {
                    metadata_setter => rule!(require(owner_badge_address));
                    metadata_setter_updater => rule!(require(owner_badge_address));
                    metadata_locker => rule!(require(owner_badge_address));
                    metadata_locker_updater => rule!(require(owner_badge_address));
                },
                init {
                    "name" => "User", updatable;
                }
            ))
            .mint_roles(mint_roles!(
                minter => rule!(require(global_caller(component_address)));
                minter_updater => rule!(require(owner_badge_address));
            ))
            .non_fungible_data_update_roles(non_fungible_data_update_roles!(
                non_fungible_data_updater => rule!(require(global_caller(component_address)));
                non_fungible_data_updater_updater => rule!(require(owner_badge_address));
            ))
            .create_with_no_initial_supply();

            let reservations_resource_manager = ResourceBuilder::new_integer_non_fungible::<ReservationNFT>(
                OwnerRole::Updatable(rule!(require(owner_badge_address)))
            )
            .metadata(metadata!(
                roles {
                    metadata_setter => rule!(require(owner_badge_address));
                    metadata_setter_updater => rule!(require(owner_badge_address));
                    metadata_locker => rule!(require(owner_badge_address));
                    metadata_locker_updater => rule!(require(owner_badge_address));
                },
                init {
                    "name" => "Reservation", updatable;
                }
            ))
            .mint_roles(mint_roles!(
                minter => rule!(require(global_caller(component_address)));
                minter_updater => rule!(require(owner_badge_address));
            ))
            .non_fungible_data_update_roles(non_fungible_data_update_roles!(
                non_fungible_data_updater => rule!(require(global_caller(component_address)));
                non_fungible_data_updater_updater => rule!(require(owner_badge_address));
            ))
            .burn_roles(burn_roles!(
                burner => rule!(require(global_caller(component_address)));
                burner_updater => rule!(require(owner_badge_address));
            ))
            .create_with_no_initial_supply();

            let arbitrators_resource_manager = ResourceBuilder::new_integer_non_fungible::<Arbitrator>(
                OwnerRole::Updatable(rule!(require(owner_badge_address)))
            )
            .metadata(metadata!(
                roles {
                    metadata_setter => rule!(require(owner_badge_address));
                    metadata_setter_updater => rule!(require(owner_badge_address));
                    metadata_locker => rule!(require(owner_badge_address));
                    metadata_locker_updater => rule!(require(owner_badge_address));
                },
                init {
                    "name" => "Arbitrator", updatable;
                }
            ))
            .mint_roles(mint_roles!(
                minter => rule!(require(global_caller(component_address)));
                minter_updater => rule!(require(owner_badge_address));
            ))
            .burn_roles(burn_roles!(
                burner => rule!(require(owner_badge_address));
                burner_updater => rule!(require(owner_badge_address));
            ))
            .withdraw_roles(withdraw_roles!(
                withdrawer => rule!(require(owner_badge_address)); // Non transferable
                withdrawer_updater => rule!(require(owner_badge_address));
            ))
            .recall_roles(recall_roles!(
                recaller => rule!(require(owner_badge_address)); // Recallable
                recaller_updater => rule!(require(owner_badge_address));
            ))
            .create_with_no_initial_supply();

            Self {
                last_user_id: 0,
                users_resource_manager: users_resource_manager,
                last_item_id: 0,
                items: KeyValueStore::new(),
                last_reservation_id: 0,
                reservations_resource_manager: reservations_resource_manager,
                arbitrators_resource_manager: arbitrators_resource_manager,
                payment_delay: 0,
                last_arbitrator_id: 0,
                min_arbitrators: 1,
            }
            .instantiate()
            .prepare_to_globalize(OwnerRole::Updatable(rule!(require(owner_badge_address))))
            .roles(roles!(
                arbitrator => rule!(require(arbitrators_resource_manager.address()));
            ))
            .with_address(address_reservation)
            .globalize()
        }

        pub fn new_user(
            &mut self,
        ) -> Bucket {
            self.last_user_id += 1;

            self.users_resource_manager.mint_non_fungible(
                &NonFungibleLocalId::integer(self.last_user_id.into()),
                User::new(self.last_user_id),
            )
        }

        fn get_user_data(
            &self,
            user_proof: Proof,
        ) -> User {
            let checked_proof = user_proof.check_with_message(
                self.users_resource_manager.address(),
                "Incorrect user proof",
            ).as_non_fungible();

            checked_proof.non_fungible::<User>().data()
        }

        pub fn new_item(
            &mut self,
            user_proof: Proof,
            minimum_reservation_period: i64,
            coin: ResourceAddress,
            min_cancellation_forewarning: i64,
        ) {
            let mut user = self.get_user_data(user_proof);

            self.last_item_id += 1;
            let item = Item::new(
                self.last_item_id,
                user.id,
                minimum_reservation_period,
                coin,
                min_cancellation_forewarning,
            );
            self.items.insert(self.last_item_id, item);

            user.owned_items.push(self.last_item_id);
            self.users_resource_manager.update_non_fungible_data(
                &NonFungibleLocalId::integer(user.id.into()),
                "owned_items",
                user.owned_items,
            );
        }

        pub fn add_or_modify_availability_interval(
            &mut self,
            user_proof: Proof,
            item_id: u64,
            start_time: i64,
            available: bool,
            price_per_minimum_reservation_period: Option<Decimal>,
        ) {
            let user_id = self.get_user_data(user_proof).id;

            let mut item = self.items.get_mut(&item_id).expect("Item not found");

            assert!(
                item.owner_id == user_id,
                "You are not the owner of this item",
            );

            item.add_or_modify_availability_interval(
                start_time,
                available,
                price_per_minimum_reservation_period,
            );
        }

        pub fn new_reservation(
            &mut self,
            user_proof: Proof,
            item_id: u64,
            start_time: i64,
            end_time: i64,
            bucket: Bucket,
        ) -> (Bucket, Bucket) {
            let user = self.get_user_data(user_proof);

            let mut item = self.items.get_mut(&item_id).expect("Item not found");

            self.last_reservation_id += 1;

            item.new_reservation(
                self.last_reservation_id,
                user.id,
                start_time,
                end_time,
                bucket,
                self.reservations_resource_manager,
            )
        }

        fn burn_reservation_nft(
            &self,
            reservation: Bucket,
        ) -> ReservationNFT {
            assert!(
                reservation.resource_address() == self.reservations_resource_manager.address(),
                "This is not a reservation NFT",
            );
            assert!(
                reservation.amount() == Decimal::ONE,
                "Cannot process multiple reservations at once",
            );

            let reservation_data = reservation
                .as_non_fungible()
                .non_fungible::<ReservationNFT>()
                .data();

            reservation.burn();

            reservation_data
        }

        pub fn reservation_cancellation_by_customer(
            &mut self,
            reservation: Bucket,
        ) -> Bucket {
            let reservation_data = self.burn_reservation_nft(reservation);

            assert!(
                reservation_data.max_cancellation_time >= Clock::current_time_rounded_to_seconds(),
                "Cannot cancel this reservation now",
            );

            self.items.get_mut(&reservation_data.item_id).expect("Item not found").get_reservation(reservation_data.id).cancellation_by_customer()
        }

        pub fn reservation_cancellation_by_owner(
            &mut self,
            user_proof: Proof,
            item_id: u64,
            reservation_id: u64,
        ) {
            let user_id = self.get_user_data(user_proof).id;

            let mut item = self.items.get_mut(&item_id).expect("Item not found");
            assert!(
                item.owner_id == user_id,
                "You are not the owner",
            );
            item.get_reservation(reservation_id).cancellation_by_owner();

            self.reservations_resource_manager.update_non_fungible_data(
                &NonFungibleLocalId::integer(reservation_id.into()),
                "status",
                ReservationStatus::OwnerCancelled,
            );
        }

        pub fn get_refund(
            &mut self,
            reservation: Bucket,
        ) -> Bucket {
            let reservation_data = self.burn_reservation_nft(reservation);

            self.items.get_mut(&reservation_data.item_id).expect("Item not found").get_reservation(reservation_data.id).get_refund()
        }

        pub fn start_dispute(
            &mut self,
            reservation_proof: Proof,
        ) {
            let checked_proof = reservation_proof.check_with_message(
                self.reservations_resource_manager.address(),
                "Incorrect reservation proof",
            ).as_non_fungible();
            let reservation_data = checked_proof.non_fungible::<ReservationNFT>().data();

            self.items.get_mut(&reservation_data.item_id).expect("Item not found").get_reservation(reservation_data.id).start_dispute();

            self.reservations_resource_manager.update_non_fungible_data(
                &NonFungibleLocalId::integer(reservation_data.id.into()),
                "status",
                ReservationStatus::Disputing,
            );
        }

        pub fn offer_partial_refund(
            &mut self,
            user_proof: Proof,
            item_id: u64,
            reservation_id: u64,
            refund_amount: Decimal,
        ) {
            let owner_id = self.get_user_data(user_proof).id;

            let mut item = self.items.get_mut(&item_id).expect("Item not found");
            assert!(
                item.owner_id == owner_id,
                "You are not the owner",
            );
            item.get_reservation(reservation_id).offer_partial_refund(refund_amount);
        }

        pub fn set_payment_delay(
            &mut self,
            payment_delay: i64,
        ) {
            assert!(
                payment_delay >= 0,
                "Negative payment_delay not allowed",
            );

            self.payment_delay = payment_delay;
        }

        pub fn get_arbitrator_badge(
            &mut self,
        ) -> Bucket {
            self.last_arbitrator_id += 1;

            self.arbitrators_resource_manager.mint_non_fungible(
                &NonFungibleLocalId::integer(self.last_arbitrator_id.into()),
                Arbitrator::new(self.last_arbitrator_id),
            )
        }

        pub fn set_min_arbitrators(
            &mut self,
            min_arbitrators: u16,
        ) {
            assert!(
                min_arbitrators > 0,
                "At least 1 arbitrator to decide",
            );

            self.min_arbitrators = min_arbitrators;
        }

        pub fn get_payment(
            &mut self,
            user_proof: Proof,
            item_id: u64,
            reservation_id: u64,
        ) -> Bucket {
            let owner_id = self.get_user_data(user_proof).id;

            let mut item = self.items.get_mut(&item_id).expect("Item not found");
            assert!(
                item.owner_id == owner_id,
                "You are not the owner",
            );
            let payment = item.get_reservation(reservation_id).get_payment(self.payment_delay);

            payment
        }

        pub fn dispute_vote(
            &mut self,
            arbitrator_proof: Proof,
            item_id: u64,
            reservation_id: u64,
            refund_percentage: Decimal,
        ) {
            let checked_proof = arbitrator_proof.check_with_message(
                self.arbitrators_resource_manager.address(),
                "Incorrect arbitrator proof",
            ).as_non_fungible();

            let arbitrator_id = checked_proof.non_fungible::<Arbitrator>().data().id;

            assert!(
                refund_percentage >= Decimal::ZERO && refund_percentage <= dec![100],
                "refund_percentage out of 0-100 range",
            );

            self.items.get_mut(&item_id).expect("Item not found")
                .get_reservation(reservation_id)
                .dispute_vote(arbitrator_id, refund_percentage, self.min_arbitrators);
        }
    }
}
