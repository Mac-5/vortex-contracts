#![cfg(test)]

//! Integration tests for the `ReputationBadge` prototype (issue #242).
//!
//! Covers the three scenarios called out in the issue's Definition of Done:
//! mint-on-promotion, burn-on-demotion, and a query view proving badge
//! state always matches the last minted/burned tier.

use crate::{Error, ReputationBadge, ReputationBadgeClient, Tier};
use soroban_sdk::{testutils::Address as _, Address, Env};

struct Ctx {
    env: Env,
    admin: Address,
    contract_id: Address,
}

impl Ctx {
    fn client(&self) -> ReputationBadgeClient<'_> {
        ReputationBadgeClient::new(&self.env, &self.contract_id)
    }
}

fn setup() -> Ctx {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, ReputationBadge);

    let ctx = Ctx {
        env,
        admin,
        contract_id,
    };
    ctx.client().initialize(&ctx.admin);
    ctx
}

#[test]
fn mint_on_promotion_sets_badge() {
    let ctx = setup();
    let solver = Address::generate(&ctx.env);

    assert_eq!(ctx.client().get_badge(&solver), None);

    ctx.client().mint_badge(&solver, &Tier::Bronze);
    assert_eq!(ctx.client().get_badge(&solver), Some(Tier::Bronze));

    // A further promotion overwrites in place rather than stacking badges.
    ctx.client().mint_badge(&solver, &Tier::Gold);
    assert_eq!(ctx.client().get_badge(&solver), Some(Tier::Gold));
}

#[test]
fn burn_on_demotion_clears_badge() {
    let ctx = setup();
    let solver = Address::generate(&ctx.env);

    ctx.client().mint_badge(&solver, &Tier::Silver);
    assert_eq!(ctx.client().get_badge(&solver), Some(Tier::Silver));

    ctx.client().burn_badge(&solver);
    assert_eq!(ctx.client().get_badge(&solver), None);
}

#[test]
fn get_badge_matches_current_tier_for_unrelated_solvers() {
    let ctx = setup();
    let solver_a = Address::generate(&ctx.env);
    let solver_b = Address::generate(&ctx.env);

    ctx.client().mint_badge(&solver_a, &Tier::Platinum);

    // solver_b never received a badge, and minting for solver_a must not
    // leak state into solver_b's record.
    assert_eq!(ctx.client().get_badge(&solver_a), Some(Tier::Platinum));
    assert_eq!(ctx.client().get_badge(&solver_b), None);
}

#[test]
#[should_panic]
fn burn_badge_without_existing_badge_panics() {
    let ctx = setup();
    let solver = Address::generate(&ctx.env);
    ctx.client().burn_badge(&solver);
}

#[test]
fn initialize_twice_is_rejected() {
    let ctx = setup();
    assert_eq!(
        ctx.client().try_initialize(&ctx.admin),
        Err(Ok(Error::AlreadyInitialized.into()))
    );
}

#[test]
fn mint_and_burn_before_initialize_are_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, ReputationBadge);
    let client = ReputationBadgeClient::new(&env, &contract_id);
    let solver = Address::generate(&env);

    assert_eq!(
        client.try_mint_badge(&solver, &Tier::Bronze),
        Err(Ok(Error::NotInitialized.into()))
    );
    assert_eq!(
        client.try_burn_badge(&solver),
        Err(Ok(Error::NotInitialized.into()))
    );
}

#[test]
fn mint_and_burn_require_admin_auth() {
    let ctx = setup();
    let solver = Address::generate(&ctx.env);
    // Drop the blanket auth mock: no signature from the admin is present.
    ctx.env.set_auths(&[]);

    assert!(ctx.client().try_mint_badge(&solver, &Tier::Gold).is_err());
    assert!(ctx.client().try_burn_badge(&solver).is_err());
    assert_eq!(ctx.client().get_badge(&solver), None);
}
