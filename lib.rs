#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod raffle {

    #[cfg(not(feature = "ink-as-dependency"))]
    use ink_storage::collections::{HashMap as StorageHashMap, Vec as StorageVec};

    use ink_env::hash::Blake2x128;

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct Raffle {
        /// The collected money would be sent to `beneficiary` when the second winner is drawn.
        beneficiary: AccountId,
        /// Participants.
        participants: StorageHashMap<AccountId, Balance>,
        /// Winner candidates.
        candidates: StorageVec<AccountId>,
        /// The winners.
        winners: StorageVec<AccountId>,
        /// The time when the raffle could be drawn.
        draw_starts_at: Timestamp,
    }

    #[ink(event)]
    pub struct Played {
        /// Who played the raffle.
        #[ink(topic)]
        who: AccountId,
        /// Transferred balance.
        #[ink(topic)]
        balance: Balance,
    }

    #[ink(event)]
    pub struct Draw {
        /// The winner of this draw.
        #[ink(topic)]
        winner: AccountId,
    }

    #[ink(event)]
    pub struct Finished {
        /// The beneficiary.
        #[ink(topic)]
        beneficiary: AccountId,
        /// Total balanced sent.
        #[ink(topic)]
        balance: Balance,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        /// Invalid deposit amount, must be between 0.01 to 0.1;
        InvalidDepositAmount,
        /// The player has already played.
        HasPlayed,
        /// Two winners drawn, raffle finished.
        Finished,
        /// Draw is not started yet.
        DrawNotStarted,
        /// Minimum player count not reached.
        NotEnoughPlayer,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    const DRAW_COUNTDOWN: Timestamp = 900000;
    const MINI_PLAYER_COUNT: u32 = 5;

    impl Raffle {
        /// Constructor that initializes the `beneficiary` value to the given address.
        #[ink(constructor)]
        pub fn new(beneficiary: AccountId) -> Self {
            Self {
                beneficiary,
                participants: StorageHashMap::new(),
                candidates: StorageVec::new(),
                winners: StorageVec::new(),
                draw_starts_at: 0,
            }
        }

        fn finished(&self) -> bool {
            self.winners.len() == 2
        }

        #[ink(message)]
        #[ink(payable)]
        pub fn play(&mut self) -> Result<()> {
            if self.finished() {
                return Err(Error::Finished);
            }

            let who = self.env().caller();
            if self.participants.get(&who).is_some() {
                return Err(Error::HasPlayed);
            }

            let balance = self.env().transferred_balance();
            if balance < 10000000000000 || balance > 100000000000000 {
                return Err(Error::InvalidDepositAmount);
            }

            self.participants.insert(who, balance);
            self.candidates.push(who);

            if self.participants.len() == 5 {
                self.draw_starts_at = Self::env().block_timestamp() + DRAW_COUNTDOWN;
            }

            self.env().emit_event(Played { who, balance });

            Ok(())
        }

        #[ink(message)]
        pub fn draw(&mut self) -> Result<()> {
            if self.finished() {
                return Err(Error::Finished);
            }

            if self.draw_starts_at == 0 || self.env().block_timestamp() < self.draw_starts_at {
                return Err(Error::DrawNotStarted);
            }

            if self.participants.len() < MINI_PLAYER_COUNT {
                return Err(Error::NotEnoughPlayer);
            }

            // the seed would unique even two valid draws in same block.
            let seed = (self.env().block_timestamp(), self.winners.len());
            let hashed_seed = self.env().hash_encoded::<Blake2x128, _>(&seed);
            let mut rand = self.env().random(&hashed_seed[..]);
            let rand_int = rand.as_mut().iter().fold(0u8, |acc, r| acc ^ r);

            let winner_index = rand_int as u32 % self.candidates.len();
            let winner = self.candidates[winner_index];

            self.winners.push(self.candidates[winner_index]);
            self.candidates.swap_remove_drop(winner_index);

            self.env().emit_event(Draw { winner });

            if self.finished() {
                // transfer all balances
                let balance = self.participants.iter().fold(0, |acc, p| acc + p.1);
                let _ = self.env().transfer(self.beneficiary, balance);

                self.env().emit_event(Finished { beneficiary: self.beneficiary, balance });
            }

            Ok(())
        }

        #[ink(message)]
        pub fn winners(&self) -> (Option<AccountId>, Option<AccountId>) {
            let first = self.winners.first().map(|w| w.clone());
            let last = self.winners.last().map(|w| w.clone());
            return (first, last)
        }

        #[ink(message)]
        pub fn beneficiary(&self) -> AccountId {
            self.beneficiary
        }
    }
}
