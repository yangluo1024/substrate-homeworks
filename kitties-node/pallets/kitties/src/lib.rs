#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        dispatch::{fmt::Debug, DispatchResult},
        pallet_prelude::*,
        traits::{Randomness, ReservableCurrency, Currency, ExistenceRequirement},
    };    
    use frame_system::pallet_prelude::*;
    use codec::{Encode, Decode};
    use sp_io::hashing::blake2_128;
    use scale_info::TypeInfo;
    use sp_runtime::traits::{MaybeDisplay, AtLeast32Bit, Bounded};

    #[derive(Encode, Decode, TypeInfo)]
    pub struct Kitty(pub [u8; 16]);

    type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
        type KittyIndex: Parameter
            + Member 
            + MaybeSerializeDeserialize 
            + Debug 
            + Default
            + MaybeDisplay
            + AtLeast32Bit
            + Copy
            + Encode;
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
        #[pallet::constant]
        type KittyDepositBase: Get<BalanceOf<Self>>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn kitties_count)]
    pub type KittiesCount<T: Config> = StorageValue<_, T::KittyIndex>;

    #[pallet::storage]
    #[pallet::getter(fn kitties)]
    pub type Kitties<T: Config> = StorageMap<_, Blake2_128Concat, T::KittyIndex, Option<Kitty>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn owner)]
    pub type Owner<T: Config> = StorageMap<_, Blake2_128Concat, T::KittyIndex, Option<T::AccountId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn price)]
    pub type Price<T: Config> = StorageMap<_, Blake2_128Concat, T::KittyIndex, Option<BalanceOf<T>>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        KittyCreate(T::AccountId, T::KittyIndex),
        KittyTransfer(T::AccountId, T::AccountId, T::KittyIndex),
        KittySale(T::AccountId, T::KittyIndex, Option<BalanceOf<T>>),
    }

    #[pallet::error]
    pub enum Error<T> {
        KittiesCountOverflow,
        NotKittyOwner,
        SameParentIndex,
        InvalidKittyIndex,
        InsufficientBalance,
        BuyFromSelf,
        KittyNotForSale,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(1_000)]
        pub fn create(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Generate kitty id and dna, checking the id is valid.
            let kitty_id = Self::get_id();
            ensure!(kitty_id != T::KittyIndex::max_value(), Error::<T>::KittiesCountOverflow);
            let dna = Self::random_value(&who);

            // Reserve for create kitty
            let deposit = T::KittyDepositBase::get();
            T::Currency::reserve(&who, deposit.clone()).map_err(|_| Error::<T>::InsufficientBalance)?;

            // Update chain's data.
            Kitties::<T>::insert(kitty_id, Some(Kitty(dna)));
            Owner::<T>::insert(kitty_id, Some(who.clone()));
            KittiesCount::<T>::put(kitty_id + 1u32.into());

            // Deposit a "KittyCreate" event.
            Self::deposit_event(Event::KittyCreate(who, kitty_id));
            Ok(())
        }

        #[pallet::weight(1_000)]
        pub fn transfer(
            origin: OriginFor<T>, 
            new_owner: T::AccountId, 
            kitty_id: T::KittyIndex,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Check caller is kitty's owner.
            ensure!(Some(who.clone()) == Owner::<T>::get(kitty_id), Error::<T>::NotKittyOwner);

            // Update the kitty's owner. (transfer to `new_owner`)
            Owner::<T>::insert(kitty_id, Some(new_owner.clone()));

            // Deposit a "KittyTransfer" event.
            Self::deposit_event(Event::KittyTransfer(who, new_owner, kitty_id));
            Ok(())
        }

        #[pallet::weight(1_000)]
        pub fn breed(
            origin: OriginFor<T>,
            kitty_id1: T::KittyIndex,
            kitty_id2: T::KittyIndex,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Ensure the two kitty are different kitties, checking they are exist.
            ensure!(kitty_id1 != kitty_id2, Error::<T>::SameParentIndex);
            let kitty1 = Self::kitties(kitty_id1).ok_or(Error::<T>::InvalidKittyIndex)?;
            let kitty2 = Self::kitties(kitty_id2).ok_or(Error::<T>::InvalidKittyIndex)?;

            // Generate kitty id and dna, checking the id is valid.
            let kitty_id = Self::get_id();
            ensure!(kitty_id != T::KittyIndex::max_value(), Error::<T>::KittiesCountOverflow);
            let dna = Self::breed_dna(&who, &kitty1, &kitty2);

            // Update chain's data.
            Kitties::<T>::insert(kitty_id, Some(Kitty(dna)));
            Owner::<T>::insert(kitty_id, Some(who.clone()));
            KittiesCount::<T>::put(kitty_id + 1u32.into());

            // Deposit a "KittyCreate" event.
            Self::deposit_event(Event::KittyCreate(who, kitty_id));
            Ok(())
        }

        #[pallet::weight(1_000)]
        pub fn sell_kitty(
            origin: OriginFor<T>,
            kitty_id: T::KittyIndex,
            price: Option<BalanceOf<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Ensure caller is the kitty owner.
            ensure!(Some(who.clone()) == Owner::<T>::get(kitty_id), Error::<T>::NotKittyOwner);

            // Update the kitty price.
            Price::<T>::insert(kitty_id, price);

            // Deposit a "KittySale" event.
            Self::deposit_event(Event::KittySale(who, kitty_id, price));
            Ok(())
        }

        #[pallet::weight(1_000)]
        pub fn buy_kitty(origin: OriginFor<T>, kitty_id: T::KittyIndex) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Ensure the kitty is exist and its owner is not the buyer.
            ensure!(Kitties::<T>::contains_key(kitty_id), Error::<T>::InvalidKittyIndex);
            let from = Owner::<T>::get(kitty_id).unwrap();
            ensure!(who.clone() != from, Error::<T>::BuyFromSelf);

            // Get the price, and do the reserve and unreserve things.
            let price = Self::price(kitty_id).ok_or(Error::<T>::KittyNotForSale)?;
            let reserve = T::KittyDepositBase::get();
            T::Currency::reserve(&who, reserve).map_err(|_| Error::<T>::InsufficientBalance)?;
            T::Currency::unreserve(&from, reserve);

            // Transfer balance to kitty owner
            T::Currency::transfer(
                &who, &from, 
                price, ExistenceRequirement::KeepAlive,
            )?;

            // Update chain's data, changing the kitty owner to caller.
            Price::<T>::remove(kitty_id);  // Not for sale.
            Owner::<T>::insert(kitty_id, Some(who.clone()));

            // Deposit a "KittyTransfer" event.
            Self::deposit_event(Event::KittyTransfer(from, who, kitty_id));
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn random_value(sender: &T::AccountId) -> [u8; 16] {
            let payload = (
                T::Randomness::random_seed(),
                &sender,
                <frame_system::Pallet<T>>::extrinsic_index(), 
            );
            payload.using_encoded(blake2_128)
        }

        pub fn get_id() -> T::KittyIndex {
            match Self::kitties_count() {
                Some(id) => id,
                None => 0u32.into(),
            }
        }

        pub fn breed_dna(who: &T::AccountId, kitty1: &Kitty, kitty2: &Kitty) -> [u8; 16] {
            let dna1 = kitty1.0;
            let dna2 = kitty2.0;
            let mut mix_dna = Self::random_value(&who);
            for i in 0..dna1.len() {
                mix_dna[i] = (mix_dna[i] & dna1[i]) | (!mix_dna[i] & dna2[i]);
            }
            mix_dna
        } 
    }
}
