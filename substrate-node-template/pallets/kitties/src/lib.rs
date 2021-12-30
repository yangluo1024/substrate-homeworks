#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use frame_support::{
        sp_runtime::traits::Hash,
        traits::{ Randomness, Currency, tokens::ExistenceRequirement },
        transactional,
    };
    use sp_io::hashing::blake2_128;
	use scale_info::TypeInfo;

    #[cfg(feature = "std")]
    use frame_support::serde::{Deserialize, Serialize};

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	// Struct for holding Kitty information.
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Kitty<T: Config> {
		pub dna: [u8; 16],  // Using 16 bytes to represent a kitty DNA.
		pub price: Option<BalanceOf<T>>,
		pub gender: Gender,
		pub owner: AccountOf<T>,
	}

	// Enum declaration for Gender.
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	pub enum Gender {
		Male,
		Female,
	}

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Configure the pallet by specifying the parameters and types it depends on.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The Currency handler for the Kitties pallet.
        type Currency: Currency<Self::AccountId>;

		/// The type of Randomness we want to specify for this pallet.
		type KittyRandomness: Randomness<Self::Hash, Self::BlockNumber>;

		/// The maximum amount of Kitties a single account can own.
		#[pallet::constant]
		type MaxKittyOwned: Get<u32>;
    }

    // Errors.
    #[pallet::error]
    pub enum Error<T> {
        /// Handles arithmetic overflow when incrementing the Kitty counter.
        KittyCntOverflow,
        /// An account cannot own more Kitties than `MaxKittyCount`.
        ExceedMaxKittyOwned,
        /// Buyer cannot be the owner.
        BuyerIsKittyOwner,
        /// Cannot transfer a kitty to its owner.
        TransferToSelf,
        /// Handles checking whether the kitty is exists.
        KittyNotExist,
        /// Handles checking that the Kitty is owned by the account transferring, buying or setting a price for it.
        NotKittyOwner,
        /// Ensures the Kitty is for sale.
        KittyNotForSale,
        /// Ensures that the buying price is greater than the asking_price.
        KittyBidPriceTooLow,
        /// Ensures that an account has enough funds to purchase a Kitty.
        NotEnoughBalance,
    }

    // Events.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new Kitty was successfully created. \[sender, kitty_id\]
        Created(T::AccountId, T::Hash),
        /// Kitty price was successfully set. \[sender, kitty_id, new_price\]
        PriceSet(T::AccountId, T::Hash, Option<BalanceOf<T>>),
        /// A Kitty was successfully transferred. \[from, to, kitty_id\]
        Transferred(T::AccountId, T::AccountId, T::Hash),
        /// A Kitty was successfully bought. \[buyer, seller, kitty_id, bid_price\]
        Bought(T::AccountId, T::AccountId, T::Hash, BalanceOf<T>),
    }


	// Storage items
    #[pallet::storage]
    #[pallet::getter(fn kitty_cnt)]
	/// Keeps track of the number of Kitties in existence.
    pub(super) type KittyCnt<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn kitties)]
	/// Stores a Kitty's unique traits, owner and price.
	pub(super) type Kitties<T: Config> = StorageMap<_, Twox64Concat, T::Hash, Kitty<T>>;

	#[pallet::storage]
	#[pallet::getter(fn kitties_owned)]
	/// Keeps track of what accounts own what Kitty.
	pub(super) type KittiesOwned<T: Config> = StorageMap<
		_, Twox64Concat, T::AccountId, BoundedVec<T::Hash, T::MaxKittyOwned>, ValueQuery>;

    // Our pallet's genesis configuration.
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub kitties: Vec<(T::AccountId, [u8; 16], Gender)>,
    }

    // Require to implement default for GenesisConfig.
    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> GenesisConfig<T> {
            GenesisConfig { kitties: vec![] }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        // when building a kitty from genesis config, we require the dna and gender to be supplied.
        fn build(&self) {
            for (acct, dna, gender) in &self.kitties {
                let _ = <Pallet<T>>::mint(acct, Some(dna.clone()), Some(gender.clone()));
            }
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}    

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new unique kitty.
        /// 
        /// The actual kitty creation is done in the `mint()` function.
        #[pallet::weight(100)]
        pub fn create_kitty(origin: OriginFor<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let kitty_id = Self::mint(&sender, None, None)?;
            // TODO: finish this function
            Ok(())
        }
    }

    //** Our helper functions.**//

    impl<T: Config> Pallet<T> {
		fn gen_gender() -> Gender {
			let random = T::KittyRandomness::random(&b"gender"[..]).0;
			match random.as_ref()[0] % 2 {
				0 => Gender::Male,
				_ => Gender::Female,
			}
		}

		fn gen_dna() -> [u8; 16] {
			let payload = (T::KittyRandomness::random(&b"dna"[..])).0;
			payload.using_encoded(blake2_128)
		}

        // Helper to mint a Kitty.
        pub fn mint(
            owner: &T::AccountId, 
            dna: Option<[u8; 16]>, 
            gender: Option<Gender>,
        ) -> Result<T::Hash, Error<T>> {
            let kitty = Kitty::<T> {
                dna: dna.unwrap_or_else(Self::gen_dna),
                price: None,
                gender: gender.unwrap_or_else(Self::gen_gender),
                owner: owner.clone(),
            };
            let kitty_id = T::Hashing::hash_of(&kitty);

            // Performs this operation first as it may fail.
            let new_cnt = Self::kitty_cnt().checked_add(1)
                .ok_or(<Error<T>>::KittyCntOverflow)?;
            
            // Performs this operation first as it may fail.
            <KittiesOwned<T>>::try_mutate(&owner, |kitty_vec| {
                kitty_vec.try_push(kitty_id)
            }).map_err(|_| <Error<T>>::ExceedMaxKittyOwned)?;

            <Kitties<T>>::insert(kitty_id, kitty);
            <KittyCnt<T>>::put(new_cnt);
            Ok(kitty_id)
        }
    }
}