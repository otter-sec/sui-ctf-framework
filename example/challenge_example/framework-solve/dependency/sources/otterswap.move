module challenge::OtterSwap {

    // ---------------------------------------------------
    // DEPENDENCIES
    // ---------------------------------------------------

    use std::vector;

    use sui::coin::{Self, Coin};
    use sui::balance::{Self, Balance, Supply};
    use sui::object::{Self, UID};
    use sui::transfer;
    use sui::tx_context::{Self, TxContext};

    // ---------------------------------------------------
    // STRUCTS
    // ---------------------------------------------------

    public struct LP<phantom X, phantom Y> has drop {}

    public struct Pool<phantom CoinTypeA, phantom CoinTypeB> has key, store {
        id: UID,
        type_a: Balance<CoinTypeA>,
        type_b: Balance<CoinTypeB>,
        lp_supply: Supply<LP<CoinTypeA, CoinTypeB>>,
        fee: u64,
    }

    // ---------------------------------------------------
    // CONSTANTS
    // ---------------------------------------------------

    const EADMIN_ONLY: u64 = 1337;
    const EINVALID_AMOUNT: u64 = 1338;
    const EALREADY_INITIALIZED: u64 = 1339;
    const EBROKEN_INVARIANT: u64 = 1340;
    const EINVALID_PARAMS: u64 = 1341;
    const ERESERVES_EMPTY: u64 = 1342;

    // ---------------------------------------------------
    // FUNCTIONS
    // ---------------------------------------------------

    public fun create_pool<CoinTypeA, CoinTypeB>( ctx: &mut TxContext ) {

        let sender = tx_context::sender(ctx);
        assert!( sender == @admin, EADMIN_ONLY );

        transfer::share_object( Pool { 
            id: object::new(ctx),
            type_a: coin::into_balance(coin::zero<CoinTypeA>(ctx)), 
            type_b: coin::into_balance(coin::zero<CoinTypeB>(ctx)), 
            lp_supply: balance::create_supply<LP<CoinTypeA, CoinTypeB>>(LP {}),
            fee: 100,
        });
    }


    public fun initialize_pool<CoinTypeA, CoinTypeB>( liquidity_pool: &mut Pool<CoinTypeA, CoinTypeB>, coin_a: Coin<CoinTypeA>, coin_b: Coin<CoinTypeB>, ctx: &mut TxContext ) {
        
        let coin_a_value = coin::value<CoinTypeA>(&coin_a);
        let coin_b_value = coin::value<CoinTypeB>(&coin_b);
        
        assert!( coin_a_value > 0 && coin_b_value > 0, EINVALID_AMOUNT );

        let pool_a_value : u64 = balance::value<CoinTypeA>(&liquidity_pool.type_a);
        let pool_b_value : u64 = balance::value<CoinTypeB>(&liquidity_pool.type_b);

        assert!( pool_a_value == 0 && pool_b_value == 0, EALREADY_INITIALIZED );

        coin::put<CoinTypeA>(&mut liquidity_pool.type_a, coin_a);
        coin::put<CoinTypeB>(&mut liquidity_pool.type_b, coin_b);

        let lp_bal = balance::increase_supply(&mut liquidity_pool.lp_supply, coin_a_value + coin_b_value);
        transfer::public_transfer(coin::from_balance(lp_bal, ctx), tx_context::sender(ctx));
    }


    public fun swap_a_b<CoinTypeA, CoinTypeB>( liquidity_pool: &mut Pool<CoinTypeA, CoinTypeB>, coin_in: Coin<CoinTypeA>, ctx: &mut TxContext ) : Coin<CoinTypeB> {

        let coin_in_value = coin::value(&coin_in);

        let balance_a : u64 = balance::value(&liquidity_pool.type_a);
        let balance_b : u64 = balance::value(&liquidity_pool.type_b);

        assert!(balance_a > 0 && balance_b > 0, ERESERVES_EMPTY);
        assert!( coin_in_value < balance_a, EINVALID_AMOUNT );

        let coin_out_value = (balance_b - (((balance_a as u128) * (balance_b as u128)) / ((balance_a as u128) + (coin_in_value as u128)) as u64));

        coin::put(&mut liquidity_pool.type_a, coin_in);
        let coin_out = coin::take(&mut liquidity_pool.type_b, coin_out_value, ctx);
        coin_out
    }

    public fun swap_b_a<CoinTypeA, CoinTypeB>( liquidity_pool: &mut Pool<CoinTypeA, CoinTypeB>, coin_in: Coin<CoinTypeB>, ctx: &mut TxContext ) : Coin<CoinTypeA> {

        let coin_in_value = coin::value(&coin_in);

        let balance_a : u64 = balance::value(&liquidity_pool.type_b);
        let balance_b : u64 = balance::value(&liquidity_pool.type_a);

        assert!(balance_a > 0 && balance_b > 0, ERESERVES_EMPTY);
        assert!( coin_in_value < balance_a, EINVALID_AMOUNT );

        let coin_out_value = ( ((balance_b as u128) * (coin_in_value as u128) / ( (balance_a as u128) + (coin_in_value as u128))) as u64);

        coin::put(&mut liquidity_pool.type_b, coin_in);
        let coin_out = coin::take(&mut liquidity_pool.type_a, coin_out_value, ctx);
        coin_out
    }


    public fun add_liquidity<CoinTypeA, CoinTypeB>( liquidity_pool: &mut Pool<CoinTypeA, CoinTypeB>, coin_a: Coin<CoinTypeA>, coin_b: Coin<CoinTypeB>, ctx: &mut TxContext ) {
        
        let coin_a_value = coin::value<CoinTypeA>(&coin_a);
        let coin_b_value = coin::value<CoinTypeB>(&coin_b);
        
        assert!( coin_a_value > 0 && coin_b_value > 0, EINVALID_AMOUNT );

        let pool_a_value = balance::value<CoinTypeA>(&liquidity_pool.type_a);
        let pool_b_value = balance::value<CoinTypeB>(&liquidity_pool.type_b);

        if (pool_a_value > pool_b_value) {
            assert!( (pool_a_value / pool_b_value) == (pool_a_value + coin_a_value ) / (pool_b_value + coin_b_value), EBROKEN_INVARIANT );
        }
        else {
            assert!( (pool_b_value / pool_a_value) == (pool_b_value + coin_b_value ) / (pool_a_value + coin_a_value), EBROKEN_INVARIANT );
        };

        coin::put(&mut liquidity_pool.type_a, coin_a);
        coin::put(&mut liquidity_pool.type_b, coin_b);

        let lp_bal = balance::increase_supply(&mut liquidity_pool.lp_supply, coin_a_value + coin_b_value);

        transfer::public_transfer(coin::from_balance(lp_bal, ctx), tx_context::sender(ctx));
    }

   
    public fun remove_liquidity<CoinTypeA, CoinTypeB>( liquidity_pool: &mut Pool<CoinTypeA, CoinTypeB>, lp: Coin<LP<CoinTypeA, CoinTypeB>>, vec: vector<u64>, ctx: &mut TxContext ) {
       
        assert!( vector::length(&vec) == 2, EINVALID_PARAMS );
        let coin_x_out = *vector::borrow(&vec, 0);
        let coin_y_out = *vector::borrow(&vec, 1);

        let lp_balance_value = coin::value<LP<CoinTypeA, CoinTypeB>>(&lp);

        assert!( lp_balance_value == coin_x_out + coin_y_out, EINVALID_AMOUNT );
        assert!( balance::value(&liquidity_pool.type_a) > coin_x_out, EINVALID_AMOUNT );
        assert!( balance::value(&liquidity_pool.type_b) > coin_y_out, EINVALID_AMOUNT );

        balance::decrease_supply(&mut liquidity_pool.lp_supply, coin::into_balance(lp));
        
        let coin_a = coin::take(&mut liquidity_pool.type_a, coin_x_out, ctx);
        let coin_b = coin::take(&mut liquidity_pool.type_b, coin_y_out, ctx);

        let sender = tx_context::sender(ctx);
        transfer::public_transfer(coin_a, sender);
        transfer::public_transfer(coin_b, sender);
    }


    public fun get_balance<CoinTypeA, CoinTypeB>(liquidity_pool: &mut Pool<CoinTypeA, CoinTypeB>): (u64, u64) {

        let pool_a_value = balance::value<CoinTypeA>(&liquidity_pool.type_a);
        let pool_b_value = balance::value<CoinTypeB>(&liquidity_pool.type_b);

        (pool_a_value, pool_b_value)
    }


    public fun get_a_price<CoinTypeA, CoinTypeB>(liquidity_pool: &mut Pool<CoinTypeA, CoinTypeB>): u64 {

        let pool_a_value = balance::value<CoinTypeA>(&liquidity_pool.type_a);
        let pool_b_value = balance::value<CoinTypeB>(&liquidity_pool.type_b);

        pool_b_value / pool_a_value
    }

    public fun get_b_price<CoinTypeA, CoinTypeB>(liquidity_pool: &mut Pool<CoinTypeA, CoinTypeB>): u64 {

        let pool_a_value = balance::value<CoinTypeA>(&liquidity_pool.type_a);
        let pool_b_value = balance::value<CoinTypeB>(&liquidity_pool.type_b);

        pool_a_value / pool_b_value
    }


    // ---------------------------------------------------
    // TESTS
    // ---------------------------------------------------

    #[test_only]
    use sui::test_scenario::{Self, next_tx};

    #[test_only]
    use sui::coin::{mint_for_testing};

    #[test_only]
    use std::debug;

    #[test_only]
    use challenge::osec::OSEC;

    #[test_only]
    use challenge::ctf::CTF;

    #[test]
    fun test_create_pool() {
        let investor = @0x111;
        let swapper = @0x222;
        let contract = @admin;

        let mut scenario_val = test_scenario::begin(@admin);
        let scenario = &mut scenario_val;

        next_tx(scenario, contract);
        {
            create_pool<CTF, OSEC>(test_scenario::ctx(scenario));
        };

        next_tx(scenario, contract);
        {
            let mut liquidity_pool = test_scenario::take_shared<Pool<CTF, OSEC>>(scenario);
            let coin_a : Coin<CTF> = mint_for_testing(100, test_scenario::ctx(scenario));
            let coin_b : Coin<OSEC> = mint_for_testing(100, test_scenario::ctx(scenario));
            initialize_pool(&mut liquidity_pool, coin_a, coin_b, test_scenario::ctx(scenario));
            test_scenario::return_shared(liquidity_pool);
        };

        next_tx(scenario, investor);
        {
            let mut liquidity_pool = test_scenario::take_shared<Pool<CTF, OSEC>>(scenario);
            let coin_a : Coin<CTF> = mint_for_testing(100, test_scenario::ctx(scenario));
            let coin_b : Coin<OSEC> = mint_for_testing(100, test_scenario::ctx(scenario));
            add_liquidity(&mut liquidity_pool, coin_a, coin_b, test_scenario::ctx(scenario));
            test_scenario::return_shared(liquidity_pool);
        };

        next_tx(scenario, swapper);
        {
            let mut liquidity_pool = test_scenario::take_shared<Pool<CTF, OSEC>>(scenario);
            let coin_a : Coin<CTF> = mint_for_testing(25, test_scenario::ctx(scenario));
            let coin_out = swap_a_b(&mut liquidity_pool, coin_a, test_scenario::ctx(scenario));
            transfer::public_transfer(coin_out, swapper);
            test_scenario::return_shared(liquidity_pool);
        };

        next_tx(scenario, investor);
        {
            let mut liquidity_pool = test_scenario::take_shared<Pool<CTF, OSEC>>(scenario);
            let vec = vector<u64>[100, 100];
            let lp = test_scenario::take_from_sender<Coin<LP<CTF, OSEC>>>(scenario);
            remove_liquidity(&mut liquidity_pool, lp, vec, test_scenario::ctx(scenario));
            test_scenario::return_shared(liquidity_pool);
        };

        test_scenario::end(scenario_val);
    }
}