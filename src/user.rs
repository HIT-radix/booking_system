//TODO: check if there's a maximum number of items a user can own

use scrypto::prelude::*;

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct NewUserEvent {
    user_id: u64,
}

#[derive(Debug, ScryptoSbor, NonFungibleData)]
pub struct User {
    pub id: u64,
    #[mutable]
    pub owned_items: Vec<u64>,
}

impl User {

    pub fn new(
        id: u64,
    ) -> User {

        Runtime::emit_event(
            NewUserEvent {
                user_id: id,
            }
        );

        Self {
            id: id,
            owned_items: vec![],
        }
    }
}

