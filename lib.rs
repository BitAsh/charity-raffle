#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod charity_raffle {

    #[cfg(not(feature = "ink-as-dependency"))]
    use ink_storage::collections::{HashMap as StorageHashMap, Vec as StorageVec};

    use ink_env::hash::Blake2x128;

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct CharityRaffle {
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
        who: AccountId,
        /// Transferred balance.
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
        /// Still in draw countdown.
        InDrawCountdown,
        /// Minimum player count not reached.
        NotEnoughPlayer,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    const DRAW_COUNTDOWN: Timestamp = 900000;
    const MINI_PLAYER_COUNT: u32 = 5;

    impl CharityRaffle {
        /// Constructor that initializes the `beneficiary` value to the given address.
        #[ink(constructor)]
        pub fn new(beneficiary: AccountId) -> Self {
            Self {
                beneficiary,
                participants: StorageHashMap::new(),
                candidates: StorageVec::new(),
                winners: StorageVec::new(),
                draw_starts_at: Self::env().block_timestamp() + DRAW_COUNTDOWN,
            }
        }

        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            Self::new(Default::default())
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

            self.env().emit_event(Played { who, balance });

            Ok(())
        }

        #[ink(message)]
        pub fn draw(&mut self) -> Result<()> {
            if self.finished() {
                return Err(Error::Finished);
            }

            if self.env().block_timestamp() < self.draw_starts_at {
                return Err(Error::InDrawCountdown);
            }

            if self.participants.len() < MINI_PLAYER_COUNT {
                return Err(Error::NotEnoughPlayer);
            }

            // the seed would unique even two valid draws in same block.
            let seed = self.env().block_timestamp() * 10 + self.winners.len() as u64;
            let hashed_seed = self.env().hash_encoded::<Blake2x128, _>(&seed.to_le_bytes());
            let rand = self.env().random(&hashed_seed[..]);
            let mut first_32_bits = [0u8; 4];
            first_32_bits.copy_from_slice(&rand.as_ref()[..4]);
            let rand_int = u32::from_le_bytes(first_32_bits);

            let winner_index = rand_int % self.candidates.len();
            self.winners.push(self.candidates[winner_index]);
            self.candidates.swap_remove_drop(winner_index);

            if self.finished() {
                // transfer all balances
                let balance = self.participants.iter().fold(0, |acc, p| acc + p.1);
                let _ = self.env().transfer(self.beneficiary, balance);
            }

            Ok(())
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// We test if the default constructor does its job.
        #[test]
        fn default_works() {
            let CharityRaffle = CharityRaffle::default();
            assert_eq!(CharityRaffle.get(), false);
        }

        /// We test a simple use case of our contract.
        #[test]
        fn it_works() {
            let mut CharityRaffle = CharityRaffle::new(false);
            assert_eq!(CharityRaffle.get(), false);
            CharityRaffle.flip();
            assert_eq!(CharityRaffle.get(), true);
        }
    }
}
