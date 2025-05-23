module challenge::router {

    // ---------------------------------------------------
    // DEPENDENCIES
    // ---------------------------------------------------

    use sui::tx_context::{Self, TxContext};
    use sui::transfer;
    use sui::coin::{Coin};

    use challenge::osec::OSEC;
    use challenge::ctf::CTF;
    use challenge::OtterSwap;
    use challenge::OtterLoan;

    // ---------------------------------------------------
    // FUNCTIONS
    // ---------------------------------------------------

    fun init(ctx: &mut TxContext) {
        create_pool<CTF, OSEC>(ctx);
    }

    public entry fun swap_a_b<CoinTypeA, CoinTypeB>( liquidity_pool: &mut OtterSwap::Pool<CoinTypeA, CoinTypeB>, coin_in: Coin<CoinTypeA>, ctx: &mut TxContext ) {
        let coin_out = OtterSwap::swap_a_b(liquidity_pool, coin_in, ctx);
        transfer::public_transfer(coin_out, tx_context::sender(ctx));
    }

    public entry fun swap_b_a<CoinTypeA, CoinTypeB>( liquidity_pool: &mut OtterSwap::Pool<CoinTypeA, CoinTypeB>, coin_in: Coin<CoinTypeB>, ctx: &mut TxContext ) {
        let coin_out = OtterSwap::swap_b_a(liquidity_pool, coin_in, ctx);
        transfer::public_transfer(coin_out, tx_context::sender(ctx));
    }    

    public entry fun add_liquidity<CoinTypeA, CoinTypeB>( liquidity_pool: &mut OtterSwap::Pool<CoinTypeA, CoinTypeB>, coin_a: Coin<CoinTypeA>, coin_b: Coin<CoinTypeB>, ctx: &mut TxContext ) {
        OtterSwap::add_liquidity(liquidity_pool, coin_a, coin_b, ctx);
    }

    public entry fun remove_liquidity<CoinTypeA, CoinTypeB>( liquidity_pool: &mut OtterSwap::Pool<CoinTypeA, CoinTypeB>, lps: Coin<OtterSwap::LP<CoinTypeA, CoinTypeB>>, vec: vector<u64>, ctx: &mut TxContext ) {
        OtterSwap::remove_liquidity(liquidity_pool, lps, vec, ctx);
    }

    public entry fun create_pool<CoinTypeA, CoinTypeB>( ctx: &mut TxContext ) {
        OtterSwap::create_pool<CoinTypeA, CoinTypeB>(ctx);
    }

    public fun loan<CoinOut>( lender: &mut OtterLoan::FlashLender<CoinOut>, amount: u64, ctx: &mut TxContext ) : (Coin<CoinOut>, OtterLoan::Receipt<CoinOut>) {
        OtterLoan::loan(lender, amount, ctx)
    }

    public fun repay<CoinIn>( lender: &mut OtterLoan::FlashLender<CoinIn>, payment: Coin<CoinIn>, receipt: OtterLoan::Receipt<CoinIn> ) {
        OtterLoan::repay(lender, payment, receipt);
    }

    public entry fun lend<CoinType>( to_lend: Coin<CoinType>, fee: u64, ctx: &mut TxContext ) {
        OtterLoan::create(to_lend, fee, ctx);
    }

    public entry fun withdraw<CoinOut>( lender: &mut OtterLoan::FlashLender<CoinOut>, admin_cap: &OtterLoan::AdminCapability, amount: u64, ctx: &mut TxContext ) {
        OtterLoan::withdraw(lender, admin_cap, amount, ctx);
    }

    public entry fun deposit<CoinIn>( lender: &mut OtterLoan::FlashLender<CoinIn>, admin_cap: &OtterLoan::AdminCapability, coins: Coin<CoinIn>, ctx: &mut TxContext ) {
        OtterLoan::deposit(lender, admin_cap, coins, ctx);
    }

    #[test_only]
    public fun init_for_testing(ctx: &mut TxContext) {
        init(ctx)
    }
}