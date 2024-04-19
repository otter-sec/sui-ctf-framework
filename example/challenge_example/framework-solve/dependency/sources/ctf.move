module challenge::ctf {

    // ---------------------------------------------------
    // DEPENDENCIES
    // ---------------------------------------------------

    use std::option;

    use sui::balance::{Self, Balance, Supply};
    use sui::tx_context::{Self, TxContext};
    use sui::object::{Self, UID};
    use sui::coin::{Self, Coin};
    use sui::transfer;
    use sui::url;
    // use std::debug;

    // ---------------------------------------------------
    // STRUCTS
    // ---------------------------------------------------

    public struct CTF has drop {}

    public struct CTFSupply<phantom CoinType> has key {
        id: UID,
        supply: Supply<CoinType>
    }

    public struct Airdrop<phantom CoinType> has key { 
        id: UID,
        coins: Balance<CoinType>,
        dropped: bool
    }

    // ---------------------------------------------------
    // CONSTANTS
    // ---------------------------------------------------

    const E_ALREADYDROPPED: u64 = 1337;

    // ---------------------------------------------------
    // FUNCTIONS
    // ---------------------------------------------------    

    fun init(witness: CTF, ctx: &mut TxContext) {
        let (mut treasury, metadata) = coin::create_currency(witness, 9, b"CTF", b"CTF", b"Capture The Flag", option::some(url::new_unsafe_from_bytes(b"https://ctftime.org/faq/#ctf-wtf")), ctx);
        transfer::public_freeze_object(metadata);

        transfer::share_object(Airdrop {
            id: object::new(ctx),
            coins: coin::into_balance(coin::mint<CTF>(&mut treasury, 250, ctx)),
            dropped: false,
        });

        let pool_liquidity = coin::mint<CTF>(&mut treasury, 500, ctx);
        transfer::public_transfer(pool_liquidity, tx_context::sender(ctx));

        let supply = coin::treasury_into_supply(treasury);

        let osec_supply = CTFSupply {
            id: object::new(ctx),
            supply
        };
        transfer::transfer(osec_supply, tx_context::sender(ctx));
    }

    public fun get_airdrop<CoinType>(airdrop: &mut Airdrop<CoinType>, ctx: &mut TxContext) : Coin<CoinType> {
        assert!(airdrop.dropped == false, E_ALREADYDROPPED);
        let airdrop_coins : Coin<CoinType> = coin::take(&mut airdrop.coins, 250, ctx);
        airdrop.dropped = true;
        airdrop_coins
    }

    public fun mint<CoinType>(sup: &mut CTFSupply<CoinType>, amount: u64, ctx: &mut TxContext): Coin<CoinType> {
        let ctfBalance = balance::increase_supply(&mut sup.supply, amount);
        coin::from_balance(ctfBalance, ctx)
    }

    public entry fun mint_to<CoinType>(sup: &mut CTFSupply<CoinType>, amount: u64, to: address, ctx: &mut TxContext) {
        let ctf = mint(sup, amount, ctx);
        transfer::public_transfer(ctf, to);
    }

    public fun burn<CoinType>(sup: &mut CTFSupply<CoinType>, c: Coin<CoinType>): u64 {
        balance::decrease_supply(&mut sup.supply, coin::into_balance(c))
    }

    #[test_only]
    public fun init_for_testing(ctx: &mut TxContext) {
        init(CTF {}, ctx)
    }

}