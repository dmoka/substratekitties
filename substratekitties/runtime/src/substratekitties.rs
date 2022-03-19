use support::{decl_storage, decl_module, StorageValue, StorageMap,
    dispatch::Result, ensure, decl_event, traits::Currency};
use system::ensure_signed;
use runtime_primitives::traits::{As, Hash, Zero};
use parity_codec::{Encode, Decode};
use rstd::cmp;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Kitty<Hash, Balance> {
    id: Hash,
    dna: Hash,
    price: Balance,
    gen: u64,
}

pub trait Trait: balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::Hash,
        <T as balances::Trait>::Balance
    {
        Created(AccountId, Hash),
        PriceSet(AccountId, Hash, Balance),
        Transferred(AccountId,AccountId,Hash),
        Bought(AccountId, AccountId, Hash,Balance),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as KittyStorage {
        KittyIdToKittyMap get(kitty): map T::Hash => Kitty<T::Hash, T::Balance>;
        KittyIdToKittyOwnerMap get(owner_of): map T::Hash => Option<T::AccountId>;

        AllKittiesList get(kitty_by_index): map u64 => T::Hash;
        AllKittiesCount get(all_kitties_count): u64;
        KittyHashToKittiesListIndexMap: map T::Hash => u64;

        OwnedKittiesListWithIdAndKittyIndexToKittyIdMap get(kitty_of_owner_by_index): map (T::AccountId, u64) => T::Hash;
        OwnerToKittiesCount get(owned_kitty_count): map T::AccountId => u64;
        OwnedKittiesIndex: map T::Hash => u64;
        
        Nonce: u64;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {

        fn deposit_event<T>() = default;

        fn create_kitty(origin) -> Result {
            let sender = ensure_signed(origin)?;

            let nonce = <Nonce<T>>::get();
            let random_hash = (<system::Module<T>>::random_seed(), &sender, nonce)
                .using_encoded(<T as system::Trait>::Hashing::hash);

            let new_kitty = Kitty {
                id: random_hash,
                dna: random_hash,
                price: <T::Balance as As<u64>>::sa(0),
                gen: 0,
            };

            Self::mint(sender, random_hash, new_kitty)?;
            
            <Nonce<T>>::mutate(|n| *n += 1);

            Ok(())
        }

        fn set_price(origin, kitty_id: T::Hash, new_price: T::Balance) -> Result{
            let sender = ensure_signed(origin)?;
            ensure!(<KittyIdToKittyMap<T>>::exists(kitty_id), "This cat does not exist");

            let owner = Self::owner_of(kitty_id).ok_or("No owner for this object")?;
            ensure!(owner == sender, "You are not the owner");

            let mut kitty = Self::kitty(kitty_id);
            kitty.price = new_price;

            <KittyIdToKittyMap<T>>::insert(kitty_id,kitty);
            
            Self::deposit_event(RawEvent::PriceSet(sender, kitty_id, new_price));

            Ok(())
        }

        fn transfer(origin, to: T::AccountId, kitty_id: T::Hash) -> Result {
            let sender = ensure_signed(origin)?;

            let owner = Self::owner_of(kitty_id).ok_or("No owner for this kitty")?;
            ensure!(owner == sender, "You do not own this kitty");

            Self::transfer_from(sender, to, kitty_id)?;

            Ok(())
        }

        // NOTE: We added this `buy_kitty` template for you
        fn buy_kitty(origin, kitty_id: T::Hash, max_price: T::Balance) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(<KittyIdToKittyMap<T>>::exists(kitty_id), "This cat does not exist");

            let owner = Self::owner_of(kitty_id).ok_or("No owner for this kitty")?;
            ensure!(owner != sender, "The owner is the same as sender");

            let mut kitty = Self::kitty(kitty_id);
            ensure!(!kitty.price.is_zero(), "The kitty price is zero, not for sure");
            ensure!(kitty.price <= max_price, "Kitty price is more than the max price");

            <balances::Module<T> as Currency<_>>::transfer(&sender, &owner, max_price)?;
            
            Self::transfer_from(owner.clone(), sender.clone(), kitty_id)
                .expect("`owner` is shown to own the kitty; \
                    `owner` must have greater than 0 kitties, so transfer cannot cause underflow; \
                    `all_kitty_count` shares the same type as `owned_kitty_count` \
                    and minting ensure there won't ever be more than `max()` kitties, \
                    which means transfer cannot cause an overflow; \
                    qed");

            kitty.price = <T::Balance as As<u64>>::sa(0);
            <KittyIdToKittyMap<T>>::insert(kitty_id,kitty);
    
            Self::deposit_event(RawEvent::Bought(sender, owner, kitty_id, max_price));

            Ok(())
        }

        // ACTION: We created this `breed_kitty` template for you
        fn breed_kitty(origin, kitty_id_1: T::Hash, kitty_id_2: T::Hash) -> Result{
            let sender = ensure_signed(origin)?;

            ensure!(<KittyIdToKittyMap<T>>::exists(kitty_id_1), "This cat does not exist");
            ensure!(<KittyIdToKittyMap<T>>::exists(kitty_id_2), "This cat does not exist");


            let nonce = <Nonce<T>>::get();
            let random_hash = (<system::Module<T>>::random_seed(), &sender, nonce)
                .using_encoded(<T as system::Trait>::Hashing::hash);

            let kitty_1 = Self::kitty(kitty_id_1);
            let kitty_2 = Self::kitty(kitty_id_2);

            // NOTE: Our gene splicing algorithm, feel free to make it your own
            let mut final_dna = kitty_1.dna;
            for (i, (dna_2_element, r)) in kitty_2.dna.as_ref().iter().zip(random_hash.as_ref().iter()).enumerate() {
                if r % 2 == 0 {
                    final_dna.as_mut()[i] = *dna_2_element;
                }
            }

            let new_kitty = Kitty {
                id: random_hash,
                dna: final_dna,
                price: <T::Balance as As<u64>>::sa(0),
                gen: rstd::cmp::max(kitty_1.gen, kitty_2.gen) + 1,
            };

            Self::mint(sender, random_hash, new_kitty)?;

            <Nonce<T>>::mutate(|n| *n += 1);

            Ok(())
        }
    }
}

// NOTE: We added this `impl` template for you
impl<T: Trait> Module<T> {
    fn mint(to: T::AccountId, kitty_id: T::Hash, new_kitty: Kitty<T::Hash, T::Balance>) -> Result {
        ensure!(!<KittyIdToKittyOwnerMap<T>>::exists(kitty_id), "Kitty already exists");

        let owned_kitty_count = Self::owned_kitty_count(&to);

        let new_owned_kitty_count = owned_kitty_count.checked_add(1)
            .ok_or("Overflow adding a new kitty to account balance")?;

        let all_kitties_count = Self::all_kitties_count();

        let new_all_kitties_count = all_kitties_count.checked_add(1)
            .ok_or("Overflow adding a new kitty to total supply")?;

        <KittyIdToKittyMap<T>>::insert(kitty_id, new_kitty);
        <KittyIdToKittyOwnerMap<T>>::insert(kitty_id, &to);

        <AllKittiesList<T>>::insert(all_kitties_count, kitty_id);
        <AllKittiesCount<T>>::put(new_all_kitties_count);
        <KittyHashToKittiesListIndexMap<T>>::insert(kitty_id, all_kitties_count);

        <OwnedKittiesListWithIdAndKittyIndexToKittyIdMap<T>>::insert((to.clone(), owned_kitty_count), kitty_id);
        <OwnerToKittiesCount<T>>::insert(&to, new_owned_kitty_count);
        <OwnedKittiesIndex<T>>::insert(kitty_id, owned_kitty_count);

        Self::deposit_event(RawEvent::Created(to, kitty_id));

        Ok(())
    }

    fn transfer_from(from: T::AccountId, to: T::AccountId, kitty_id: T::Hash) -> Result {
        let owner = Self::owner_of(kitty_id).ok_or("No owner for this kitty")?;
        ensure!(owner == from, "You do not own this kitty");

        let owned_kitty_count_from = Self::owned_kitty_count(&from);
        let owned_kitty_count_to = Self::owned_kitty_count(&to);

        let new_owned_kitty_count_to = owned_kitty_count_to.checked_add(1).ok_or("Overflow while adding new kitty")?;
        let new_owned_kitty_count_from = owned_kitty_count_from.checked_sub(1).ok_or("Underflow while removing a kitty")?;

        // NOTE: This is the "swap and pop" algorithm we have added for you
        //       We use our storage items to help simplify the removal of elements from the OwnedKittiesArray
        //       We switch the last element of OwnedKittiesArray with the element we want to remove
        let kitty_index = <OwnedKittiesIndex<T>>::get(kitty_id);
        if kitty_index != new_owned_kitty_count_from { //If the index is not already the last element in the array
            let last_kitty_id = <OwnedKittiesListWithIdAndKittyIndexToKittyIdMap<T>>::get((from.clone(), new_owned_kitty_count_from));
            <OwnedKittiesListWithIdAndKittyIndexToKittyIdMap<T>>::insert((from.clone(), kitty_index), last_kitty_id);
            <OwnedKittiesIndex<T>>::insert(last_kitty_id, kitty_index);
        }
        
        <KittyIdToKittyOwnerMap<T>>::insert(&kitty_id, &to);
        <OwnedKittiesIndex<T>>::insert(&kitty_id, owned_kitty_count_to);

        <OwnedKittiesListWithIdAndKittyIndexToKittyIdMap<T>>::remove((from.clone(),new_owned_kitty_count_from));
        <OwnedKittiesListWithIdAndKittyIndexToKittyIdMap<T>>::insert((to.clone(),owned_kitty_count_to),kitty_id);

        <OwnerToKittiesCount<T>>::insert(&from,new_owned_kitty_count_from);
        <OwnerToKittiesCount<T>>::insert(&to,new_owned_kitty_count_to);
        
        Self::deposit_event(RawEvent::Transferred(from,to,kitty_id));

        Ok(())
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    // Import a bunch of dependencies from substrate core. All needed for some parts of the code.
    use support::{impl_outer_origin, assert_ok, assert_noop};
    use runtime_io::{with_externalities, TestExternalities};
    use primitives::{H256, Blake2Hasher};
    use runtime_primitives::{
        BuildStorage,
        traits::{BlakeTwo256, IdentityLookup},
        testing::{Digest, DigestItem, Header}
    };

    impl_outer_origin! {
        pub enum Origin for KittiesTest {}
    }

    #[derive(Clone, Eq, PartialEq)]
    pub struct KittiesTest;

    impl system::Trait for KittiesTest {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type Digest = Digest;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = ();
        type Log = DigestItem;
    }
    
    impl balances::Trait for KittiesTest {
        type Balance = u64;
        type OnFreeBalanceZero = ();
        type OnNewAccount = ();
        type Event = ();
        type TransactionPayment = ();
        type TransferPayment = ();
        type DustRemoval = ();
    }

    impl super::Trait for KittiesTest {
        type Event = ();
    }

    type Kitties = super::Module<KittiesTest>;

    fn build_ext() -> TestExternalities<Blake2Hasher> {
        let mut t = system::GenesisConfig::<KittiesTest>::default().build_storage().unwrap().0;
        t.extend(balances::GenesisConfig::<KittiesTest>::default().build_storage().unwrap().0);
        // t.extend(GenesisConfig::<KittiesTest>::default().build_ext().unwrap().0);
        t.into()
    }

    fn check_that_kitty_is_owned_by_account(kitty_id_hash: H256, account: u64) {
        assert_eq!(Kitties::owner_of(kitty_id_hash), Some(account));

        let other_hash = Kitties::kitty_of_owner_by_index((account, 0));
        assert_eq!(kitty_id_hash, other_hash);
    }

    #[test]
    fn create_kitty_should_work() {
        with_externalities(&mut build_ext(), || {
            //Arrange
            let account = 10;
            let account_without_kitty = 5;

            //Act
            let create_kitty_result = Kitties::create_kitty(Origin::signed(account));

            //Assert
            assert_ok!(create_kitty_result);
            assert_eq!(Kitties::all_kitties_count(), 1);
            assert_eq!(Kitties::owned_kitty_count(account), 1);
            assert_eq!(Kitties::owned_kitty_count(account_without_kitty), 0);

            let hash = Kitties::kitty_by_index(0);
            check_that_kitty_is_owned_by_account(hash, account)

        })
    }

    #[test]
    fn transfer_kitty_should_work() {
        with_externalities(&mut build_ext(), || {
            //Arrange
            let account_1 = 1;
            let account_2 = 2;
            let create_kitty_result = Kitties::create_kitty(Origin::signed(account_1));
            let kitty_id = Kitties::kitty_by_index(0);

            //Act
            let tansfer_result = Kitties::transfer(
                Origin::signed(account_1), 
                account_2, 
                kitty_id);
            
            //Assert
            assert_ok!(tansfer_result);
            assert_eq!(Kitties::owned_kitty_count(account_1), 0);
            assert_eq!(Kitties::owned_kitty_count(account_2), 1);
            check_that_kitty_is_owned_by_account(kitty_id, account_2)
        })
    }

    #[test]
    fn should_return_error_when_account_transfers_not_owned_kitty() {
        with_externalities(&mut build_ext(), || {
            //Arrange
            let account_1 = 1;
            let account_2 = 2;
            let create_kitty_result = Kitties::create_kitty(Origin::signed(account_2));
            let kitty_id = Kitties::kitty_by_index(0);

            //Act
            let tansfer_result = Kitties::transfer(
                Origin::signed(account_1), 
                account_2, 
                kitty_id);
            
            //Assert
            assert_noop!(tansfer_result, "You do not own this kitty");
        })    
    }

    
    #[test]
    fn set_price_should_work() {
        with_externalities(&mut build_ext(), || {
            //Arrange
            let account = 1;
            Kitties::create_kitty(Origin::signed(account));
            let kitty_id = Kitties::kitty_by_index(0);

            //Act
            let set_price_result = Kitties::set_price(
                Origin::signed(account), 
                kitty_id, 
                100);
            
            //Assert
            assert_ok!(set_price_result);
            let kitty = Kitties::kitty(kitty_id);
            assert_eq!(kitty.price, 100);
        })    
    }

    #[test]
    fn should_return_error_when_setting_price_on_unknown_kitty() {
        with_externalities(&mut build_ext(), || {
            //Arrange
            let account = 1;
            let non_existing_kitty_id =  H256::zero();

            //Act
            let set_price_result = Kitties::set_price(
                Origin::signed(account), 
                non_existing_kitty_id, 
                100);
            
            //Assert
            assert_noop!(set_price_result, "This cat does not exist");
        })    
    }

    #[test]
    fn should_return_error_when_setting_price_not_owned_kitty() {
        with_externalities(&mut build_ext(), || {
            //Arrange
            let account_1 = 1;
            let account_2 = 2;
            Kitties::create_kitty(Origin::signed(account_1));
            let kitty_id = Kitties::kitty_by_index(0);

            //Act
            let set_price_result = Kitties::set_price(
                Origin::signed(account_2), 
                kitty_id, 
                100);
            
            //Assert
            assert_noop!(set_price_result, "You are not the owner");
        })    
    }

}