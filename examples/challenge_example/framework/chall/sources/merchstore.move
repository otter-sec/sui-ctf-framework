module challenge::merch_store {

    // ---------------------------------------------------
    // DEPENDENCIES
    // ---------------------------------------------------

    use sui::event;
    use sui::transfer;
    use std::type_name;
    use sui::coin::{Self, Coin};
    use sui::tx_context::{Self, TxContext};

    use challenge::osec::OSEC;
    // use std::debug;

    // ---------------------------------------------------
    // STRUCTS
    // ---------------------------------------------------

    public struct Flag has key, store {
        id: UID,
        user: address,
        flag: bool
    }

    public struct Hoodie has key, store {
        id: UID,
        user: address,
        flag: bool
    }

    public struct Tshirt has key, store {
        id: UID,
        user: address,
        flag: bool
    }

    // ---------------------------------------------------
    // CONSTANTS
    // ---------------------------------------------------

    const EINVALID_AMOUNT: u64 = 1337;
    const EINVALID_ITEM: u64 = 1338;

    // ---------------------------------------------------
    // FUNCTIONS
    // ---------------------------------------------------

    public entry fun buy_flag<CoinType>(coins: Coin<CoinType>, ctx: &mut TxContext) {
        assert!(type_name::get<CoinType>() == type_name::get<OSEC>(), 0);
        assert!(coin::value(&coins) == 499, EINVALID_AMOUNT);

        transfer::public_transfer(coins, @admin);

        transfer::public_transfer(Flag {
            id: object::new(ctx),
            user: tx_context::sender(ctx),
            flag: true
        }, tx_context::sender(ctx));
    }

    public entry fun has_flag(flag: &mut Flag) {
        assert!(flag.flag == true, EINVALID_ITEM);
    }

    public entry fun buy_tshirt<CoinType>(coins: Coin<CoinType>, ctx: &mut TxContext) {
        assert!(type_name::get<CoinType>() == type_name::get<OSEC>(), 0);
        assert!(coin::value(&coins) == 250, EINVALID_AMOUNT);

        transfer::public_transfer(coins, @admin);

        transfer::public_transfer(Tshirt {
            id: object::new(ctx),
            user: tx_context::sender(ctx),
            flag: true
        }, tx_context::sender(ctx));
    }

    public entry fun has_tshirt(tshirt: &mut Tshirt) {
        assert!(tshirt.flag == true, EINVALID_ITEM);
    }

    public entry fun buy_hoodie<CoinType>(coins: Coin<CoinType>, ctx: &mut TxContext) {
        assert!(type_name::get<CoinType>() == type_name::get<OSEC>(), 0);
        assert!(coin::value(&coins) == 167, EINVALID_AMOUNT);

        transfer::public_transfer(coins, @admin);

        transfer::public_transfer(Hoodie {
            id: object::new(ctx),
            user: tx_context::sender(ctx),
            flag: true
        }, tx_context::sender(ctx));
    }

    public entry fun has_hoodie(hoodie: &mut Hoodie) {
        assert!(hoodie.flag == true, EINVALID_ITEM);
    }

}