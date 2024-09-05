use scrypto::prelude::*;

#[derive(ScryptoSbor, ScryptoEvent)]
pub struct NewArbitratorEvent {
    arbitrator_id: u64,
}

#[derive(Debug, ScryptoSbor, NonFungibleData)]
pub struct Arbitrator {
    pub id: u64,
}

impl Arbitrator {

    pub fn new(
        id: u64,
    ) -> Arbitrator {

        Runtime::emit_event(
            NewArbitratorEvent {
                arbitrator_id: id,
            }
        );

        Self {
            id: id,
        }
    }
}
