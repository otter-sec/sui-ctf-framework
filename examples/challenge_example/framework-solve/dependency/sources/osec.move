module challenge::osec {

    // ---------------------------------------------------
    // DEPENDENCIES
    // ---------------------------------------------------

    use std::option;

    use sui::tx_context::{Self, TxContext};
    use sui::balance::{Self, Supply};
    use sui::object::{Self, UID};
    use sui::coin::{Self, Coin};
    use sui::transfer;
    use sui::url;
    // use std::debug;

    // ---------------------------------------------------
    // STRUCTS
    // ---------------------------------------------------

    public struct OSEC has drop {}

    public struct OsecSuply<phantom CoinType> has key {
        id: UID,
        supply: Supply<CoinType>
    }

    fun init(witness: OSEC, ctx: &mut TxContext) {
        let (mut treasury, metadata) = coin::create_currency(witness, 9, b"OSEC", b"Osec", b"Just Anotter coin", option::some(url::new_unsafe_from_bytes(b"https://osec.io/")), ctx);
        transfer::public_freeze_object(metadata);

        let pool_liquidity = coin::mint<OSEC>(&mut treasury, 500, ctx);
        transfer::public_transfer(pool_liquidity, tx_context::sender(ctx));

        let supply = coin::treasury_into_supply(treasury);

        let osec_supply = OsecSuply {
            id: object::new(ctx),
            supply
        };
        transfer::transfer(osec_supply, tx_context::sender(ctx));
    }

    public fun mint<CoinType>(sup: &mut OsecSuply<CoinType>, amount: u64, ctx: &mut TxContext): Coin<CoinType> {
        let osecBalance = balance::increase_supply(&mut sup.supply, amount);
        coin::from_balance(osecBalance, ctx)
    }

    public entry fun mint_to<CoinType>(sup: &mut OsecSuply<CoinType>, amount: u64, to: address, ctx: &mut TxContext) {
        let osec = mint(sup, amount, ctx);
        transfer::public_transfer(osec, to);
    }

    public fun burn<CoinType>(sup: &mut OsecSuply<CoinType>, c: Coin<CoinType>): u64 {
        balance::decrease_supply(&mut sup.supply, coin::into_balance(c))
    }

    #[test_only]
    public fun init_for_testing(ctx: &mut TxContext) {
        init(OSEC {}, ctx)
    }

}