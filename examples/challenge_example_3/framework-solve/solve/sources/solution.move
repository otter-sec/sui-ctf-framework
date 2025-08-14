module solution::solution {
    use challenge::interactive_ctf::{Self, Challenge, UserProgress};
    use sui::tx_context::TxContext;

    // Step 1: Complete first challenge step
    public entry fun solve_step_one(progress: &mut UserProgress) {
        interactive_ctf::step_one(progress, 100);
    }
    
    // Step 2: Complete second challenge step with the secret list
    public entry fun solve_step_two(progress: &mut UserProgress, challenge: &mut Challenge) {
        let list = vector[1u8, 3u8, 3u8, 7u8];
        interactive_ctf::step_two(progress, challenge, list);
    }
    
    // Step 3: Complete third challenge step with the secret number
    public entry fun solve_step_three(progress: &mut UserProgress, challenge: &Challenge) {
        interactive_ctf::step_three(progress, challenge, 42);
    }
    
    // Final: Complete the challenge
    public entry fun complete_challenge(progress: &mut UserProgress, challenge: &mut Challenge) {
        interactive_ctf::final_solve(progress, challenge);
    }
    
    // All-in-one solution for testing
    public entry fun solve_all(
        progress: &mut UserProgress,
        challenge: &mut Challenge,
        ctx: &mut TxContext
    ) {
        // Step 1: Call step_one with value 100
        interactive_ctf::step_one(progress, 100);
        
        // Step 2: Call step_two with list [1, 3, 3, 7]
        let list = vector[1u8, 3u8, 3u8, 7u8];
        interactive_ctf::step_two(progress, challenge, list);
        
        // Step 3: Call step_three with secret value 42
        interactive_ctf::step_three(progress, challenge, 42);
        
        // Final solve
        interactive_ctf::final_solve(progress, challenge);
    }
}