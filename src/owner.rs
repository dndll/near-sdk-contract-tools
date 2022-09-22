//! Owner pattern
#![allow(missing_docs)] // #[ext_contract(...)] does not play nicely with clippy

use near_sdk::{env, ext_contract, require, AccountId};

use crate::{slot::Slot, standard::nep297::Event};

const ONLY_OWNER_FAIL_MESSAGE: &str = "Owner only";
const OWNER_INIT_FAIL_MESSAGE: &str = "Owner already initialized";
const NO_OWNER_FAIL_MESSAGE: &str = "No owner";
const ONLY_PROPOSED_OWNER_FAIL_MESSAGE: &str = "Proposed owner only";
const NO_PROPOSED_OWNER_FAIL_MESSAGE: &str = "No proposed owner";

/// Events emitted by function calls on an ownable contract
pub mod event {
    use near_sdk::AccountId;

    use crate::event;
    /// Emitted when the current owner of the contract changes
    #[event(
        standard = "x-own",
        version = "1.0.0",
        crate = "crate",
        macros = "near_contract_tools_macros"
    )]
    #[derive(Debug, Clone)]
    pub struct Transfer {
        /// Former owner of the contract. Will be `None` if the contract is being initialized.
        pub old: Option<AccountId>,
        /// The new owner of the contract. Will be `None` if ownership is renounced.
        pub new: Option<AccountId>,
    }
    /// Emitted when the proposed owner of the contract changes
    #[event(
        standard = "x-own",
        version = "1.0.0",
        crate = "crate",
        macros = "near_contract_tools_macros"
    )]
    #[derive(Debug, Clone)]
    pub struct Propose {
        /// Old proposed owner.
        pub old: Option<AccountId>,
        /// New proposed owner.
        pub new: Option<AccountId>,
    }
}

/// A contract with an owner
pub trait Owner {
    /// Storage root
    fn root() -> Slot<()>;

    /// Storage slot for initialization state
    fn slot_is_initialized() -> Slot<bool> {
        Self::root().field(b"i")
    }

    /// Storage slot for owner account ID
    fn slot_owner() -> Slot<AccountId> {
        Self::root().field(b"o")
    }

    /// Storage slot for proposed owner account ID
    fn slot_proposed_owner() -> Slot<AccountId> {
        Self::root().field(b"p")
    }

    /// Updates the current owner and emits relevant event
    fn update_owner(&mut self, new: Option<AccountId>) {
        let owner = Self::slot_owner();
        let old = owner.read();
        if old != new {
            event::Transfer {
                old,
                new: new.clone(),
            }
            .emit();
            self.update_owner_unchecked(new);
        }
    }

    /// Updates proposed owner and emits relevant event
    fn update_proposed(&mut self, new: Option<AccountId>) {
        let proposed_owner = Self::slot_proposed_owner();
        let old = proposed_owner.read();
        if old != new {
            event::Propose {
                old,
                new: new.clone(),
            }
            .emit();
            self.update_proposed_unchecked(new);
        }
    }

    /// Updates the current owner without any checks or emitting events
    fn update_owner_unchecked(&mut self, new: Option<AccountId>) {
        let mut owner = Self::slot_owner();
        owner.set(new.as_ref());
    }

    /// Updates proposed owner without any checks or emitting events
    fn update_proposed_unchecked(&mut self, new: Option<AccountId>) {
        let mut proposed_owner = Self::slot_proposed_owner();
        proposed_owner.set(new.as_ref());
    }

    /// Initializes the contract owner. Can only be called once.
    ///
    /// Emits an `OwnerEvent::Transfer` event.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::{AccountId, near_bindgen};
    /// use near_contract_tools::{Owner, owner::Owner};
    ///
    /// #[derive(Owner)]
    /// #[near_bindgen]
    /// struct Contract {}
    ///
    /// #[near_bindgen]
    /// impl Contract {
    ///     pub fn new(owner_id: AccountId) -> Self {
    ///         let mut contract = Self {};
    ///
    ///         Owner::init(&mut contract, &owner_id);
    ///
    ///         contract
    ///     }
    /// }
    /// ```
    fn init(&mut self, owner_id: &AccountId) {
        require!(
            !Self::slot_is_initialized().exists(),
            OWNER_INIT_FAIL_MESSAGE,
        );

        Self::slot_is_initialized().write(&true);
        Self::slot_owner().write(owner_id);

        event::Transfer {
            old: None,
            new: Some(owner_id.clone()),
        }
        .emit();
    }

    /// Requires the predecessor to be the owner
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::{AccountId, near_bindgen};
    /// use near_contract_tools::{Owner, owner::Owner};
    ///
    /// #[derive(Owner)]
    /// #[near_bindgen]
    /// struct Contract {}
    ///
    /// #[near_bindgen]
    /// impl Contract {
    ///     pub fn owner_only(&self) {
    ///         Self::require_owner();
    ///
    ///         // ...
    ///     }
    /// }
    /// ```
    fn require_owner() {
        require!(
            &env::predecessor_account_id()
                == Self::slot_owner()
                    .read()
                    .as_ref()
                    .unwrap_or_else(|| env::panic_str(NO_OWNER_FAIL_MESSAGE)),
            ONLY_OWNER_FAIL_MESSAGE,
        );
    }

    /// Removes the contract's owner. Can only be called by the current owner.
    ///
    /// Emits an `OwnerEvent::Transfer` event, and an `OwnerEvent::Propose`
    /// event if there is a currently proposed owner.
    fn renounce_owner(&mut self) {
        Self::require_owner();

        self.update_proposed(None);
        self.update_owner(None);
    }

    /// Prepares the contract to change owners, setting the proposed owner to
    /// the provided account ID. Can only be called by the current owner.
    ///
    /// Emits an `OwnerEvent::Propose` event.
    ///
    /// The currently proposed owner may be reset by calling this function with
    /// the argument `None`.
    fn propose_owner(&mut self, account_id: Option<AccountId>) {
        Self::require_owner();

        self.update_proposed(account_id);
    }

    /// Sets new owner equal to proposed owner. Can only be called by proposed
    /// owner.
    ///
    /// Emits events corresponding to the transfer of ownership and reset of the
    /// proposed owner.
    fn accept_owner(&mut self) {
        let proposed_owner = Self::slot_proposed_owner()
            .take()
            .unwrap_or_else(|| env::panic_str(NO_PROPOSED_OWNER_FAIL_MESSAGE));

        require!(
            env::predecessor_account_id() == proposed_owner,
            ONLY_PROPOSED_OWNER_FAIL_MESSAGE,
        );

        event::Propose {
            old: Some(proposed_owner.clone()),
            new: None,
        }
        .emit();

        self.update_owner(Some(proposed_owner));
    }
}

/// Externally-accessible functions for `Owner`
#[ext_contract(ext_owner)]
pub trait OwnerExternal {
    /// Returns the account ID of the current owner
    fn own_get_owner(&self) -> Option<AccountId>;

    /// Returns the account ID that the current owner has proposed take over ownership
    fn own_get_proposed_owner(&self) -> Option<AccountId>;

    /// Current owner may call this function to renounce ownership, setting
    /// current owner to `None`.
    ///
    /// **WARNING**: Once this function has been called, this implementation
    /// does not provide a way for the contract to have an owner again!
    fn own_renounce_owner(&mut self);

    /// Propose a new owner. Can only be called by the current owner
    fn own_propose_owner(&mut self, account_id: Option<AccountId>);

    /// The proposed owner may call this function to accept ownership from the
    /// previous owner
    fn own_accept_owner(&mut self);
}

#[cfg(test)]
mod tests {
    use near_sdk::{near_bindgen, test_utils::VMContextBuilder, testing_env, AccountId};

    use crate::{
        owner::{Owner, OwnerExternal},
        Owner,
    };

    #[derive(Owner)]
    #[owner(crate = "crate")]
    #[near_bindgen]
    struct Contract {}

    #[near_bindgen]
    impl Contract {
        #[init]
        pub fn new(owner_id: AccountId) -> Self {
            let mut contract = Self {};

            Owner::init(&mut contract, &owner_id);

            contract
        }

        pub fn owner_only(&self) {
            Self::require_owner();
        }
    }

    #[test]
    fn require_owner() {
        let owner_id: AccountId = "owner".parse().unwrap();

        let contract = Contract::new(owner_id.clone());

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner_id)
            .build());

        contract.owner_only();
    }

    #[test]
    #[should_panic(expected = "Owner only")]
    fn require_owner_fail() {
        let owner_id: AccountId = "owner".parse().unwrap();

        let contract = Contract::new(owner_id);

        let alice: AccountId = "alice".parse().unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(alice)
            .build());

        contract.owner_only();
    }

    #[test]
    fn renounce_owner() {
        let owner_id: AccountId = "owner".parse().unwrap();

        let mut contract = Contract::new(owner_id.clone());
        assert_eq!(contract.own_get_owner(), Some(owner_id.clone()));
        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner_id)
            .attached_deposit(1)
            .build());
        contract.own_renounce_owner();
        assert_eq!(contract.own_get_owner(), None);
    }

    #[test]
    fn propose_owner() {
        let owner_id: AccountId = "owner".parse().unwrap();
        let mut contract = Contract::new(owner_id.clone());

        let proposed_owner: AccountId = "proposed".parse().unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner_id)
            .attached_deposit(1)
            .build());

        assert_eq!(contract.own_get_proposed_owner(), None);

        contract.own_propose_owner(Some(proposed_owner.clone()));

        assert_eq!(contract.own_get_proposed_owner(), Some(proposed_owner));
    }

    #[test]
    #[should_panic(expected = "Owner only")]
    fn propose_owner_unauthorized() {
        let owner_id: AccountId = "owner".parse().unwrap();
        let mut contract = Contract::new(owner_id);

        let proposed_owner: AccountId = "proposed".parse().unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(proposed_owner.clone())
            .attached_deposit(1)
            .build());

        contract.own_propose_owner(Some(proposed_owner));
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn propose_owner_no_deposit() {
        let owner_id: AccountId = "owner".parse().unwrap();
        let mut contract = Contract::new(owner_id.clone());

        let proposed_owner: AccountId = "proposed".parse().unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner_id)
            .build());

        contract.own_propose_owner(Some(proposed_owner));
    }

    #[test]
    fn accept_owner() {
        let owner_id: AccountId = "owner".parse().unwrap();

        let mut contract = Contract::new(owner_id.clone());

        let proposed_owner: AccountId = "proposed".parse().unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner_id)
            .attached_deposit(1)
            .build());

        contract.own_propose_owner(Some(proposed_owner.clone()));

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(proposed_owner.clone())
            .attached_deposit(1)
            .build());

        contract.own_accept_owner();

        assert_eq!(contract.own_get_owner(), Some(proposed_owner));
        assert_eq!(contract.own_get_proposed_owner(), None);
    }

    #[test]
    #[should_panic(expected = "Proposed owner only")]
    fn accept_owner_unauthorized() {
        let owner_id: AccountId = "owner".parse().unwrap();

        let mut contract = Contract::new(owner_id.clone());

        let proposed_owner: AccountId = "proposed".parse().unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner_id)
            .attached_deposit(1)
            .build());

        contract.own_propose_owner(Some(proposed_owner));

        let third_party: AccountId = "third".parse().unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(third_party)
            .attached_deposit(1)
            .build());

        contract.own_accept_owner();
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn accept_owner_no_deposit() {
        let owner_id: AccountId = "owner".parse().unwrap();

        let mut contract = Contract::new(owner_id.clone());

        let proposed_owner: AccountId = "proposed".parse().unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner_id)
            .attached_deposit(1)
            .build());

        contract.own_propose_owner(Some(proposed_owner.clone()));

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(proposed_owner)
            .build());

        contract.own_accept_owner();
    }

    #[test]
    fn update_owner_unchecked() {
        let owner_id: AccountId = "owner".parse().unwrap();

        let mut contract = Contract::new(owner_id);

        let new_owner: AccountId = "new_owner".parse().unwrap();

        contract.update_owner_unchecked(Some(new_owner.clone()));

        assert_eq!(contract.own_get_owner(), Some(new_owner));
        assert_eq!(contract.own_get_proposed_owner(), None);
    }
    #[test]
    fn update_proposed_unchecked() {
        let owner_id: AccountId = "owner".parse().unwrap();

        let mut contract = Contract::new(owner_id.clone());

        let proposed_owner: AccountId = "proposed".parse().unwrap();

        contract.update_proposed_unchecked(Some(proposed_owner.clone()));

        assert_eq!(contract.own_get_owner(), Some(owner_id));
        assert_eq!(contract.own_get_proposed_owner(), Some(proposed_owner));
    }
}
