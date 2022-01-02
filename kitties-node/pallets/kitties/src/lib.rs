#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        dispatch::DispatchResult,
        pallet_prelude::*,
        traits::Randomness, Blake2_128Concat,
    };    
    use frame_system::pallet_prelude::*;
    use codec::{Encode, Decode};
    use sp_io::hashing::blake2_128;
    use scale_info::TypeInfo;

    #[derive(Encode, Decode, TypeInfo)]
    pub struct Kitty(pub [u8; 16]);

    type KittyIndex = u32;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        KittyCreate(T::AccountId, KittyIndex),
        KittyTransfer(T::AccountId, T::AccountId, KittyIndex),
    }

    #[pallet::storage]
    #[pallet::getter(fn kitties_count)]
    pub type KittiesCount<T> = StorageValue<_, u32>;

    #[pallet::storage]
    #[pallet::getter(fn kitties)]
    pub type Kitties<T> = StorageMap<_, Blake2_128Concat, KittyIndex, Option<Kitty>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn owner)]
    pub type Owner<T: Config> = StorageMap<_, Blake2_128Concat, KittyIndex, Option<T::AccountId>, ValueQuery>;

    #[pallet::error]
    pub enum Error<T> {
        KittiesCountOverflow,
        NotKittyOwner,
        SameParentIndex,
        InvalidKittyIndex,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        pub fn create(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Generate kitty id and dna, checking the id is valid.
            let kitty_id = Self::gen_id()?;
            let dna = Self::random_value(&who);

            // Update chain's data.
            Kitties::<T>::insert(kitty_id, Some(Kitty(dna)));
            Owner::<T>::insert(kitty_id, Some(who.clone()));
            KittiesCount::<T>::put(kitty_id + 1);

            // Deposit a "KittyCreate" event.
            Self::deposit_event(Event::KittyCreate(who, kitty_id));
            Ok(())
        }

        #[pallet::weight(0)]
        pub fn transfer(
            origin: OriginFor<T>, 
            new_owner: T::AccountId, 
            kitty_id: KittyIndex,
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

        #[pallet::weight(0)]
        pub fn breed(
            origin: OriginFor<T>,
            kitty_id1: KittyIndex,
            kitty_id2: KittyIndex,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Ensure the two kitty are different kitties, checking they are exist.
            ensure!(kitty_id1 != kitty_id2, Error::<T>::SameParentIndex);
            let kitty1 = Self::kitties(kitty_id1).ok_or(Error::<T>::InvalidKittyIndex)?;
            let kitty2 = Self::kitties(kitty_id2).ok_or(Error::<T>::InvalidKittyIndex)?;

            // Generate kitty id and dna, checking the id is valid.
            let kitty_id = Self::gen_id()?;
            let dna = Self::breed_dna(&who, &kitty1, &kitty2);

            // Update chain's data.
            Kitties::<T>::insert(kitty_id, Some(Kitty(dna)));
            Owner::<T>::insert(kitty_id, Some(who.clone()));
            KittiesCount::<T>::put(kitty_id + 1);

            // Deposit a "KittyCreate" event.
            Self::deposit_event(Event::KittyCreate(who, kitty_id));
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn random_value(sender: &T::AccountId) -> [u8; 16] {
            let payload = (
                T::Randomness::random_seed(),
                &sender,
                <frame_system::Pallet<T>>::extrinsic_index(), 
            );
            payload.using_encoded(blake2_128)
        }

        fn gen_id() -> Result<KittyIndex, Error<T>> {
            let kitty_id = match Self::kitties_count() {
                Some(id) => {
                    ensure!(id != KittyIndex::max_value(), Error::<T>::KittiesCountOverflow);
                    id
                },
                None => { 1 },
            };
            Ok(kitty_id)
        }

        fn breed_dna(who: &T::AccountId, kitty1: &Kitty, kitty2: &Kitty) -> [u8; 16] {
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
