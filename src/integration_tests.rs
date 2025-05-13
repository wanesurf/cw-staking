#[cfg(test)]
mod tests {
    use crate::helpers::CwTemplateContract;
    use crate::msg::InstantiateMsg;
    use crate::msg::{
        ExecuteMsg as CounterExecMsg, GetCountResponse as CounterResponse,
        InstantiateMsg as CounterInitMsg, QueryMsg as CounterQueryMsg,
    };
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::{
        coin, coins, Addr, BlockInfo, Coin, CosmosMsg, Decimal, Empty, Querier, StakingMsg,
        Uint128, Validator,
    };
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor, IntoAddr};

    pub fn contract_template() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    const USER: &str = "USER";
    const ADMIN: &str = "ADMIN";
    const NATIVE_DENOM: &str = "denom";

    fn mock_app() -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &MockApi::default().addr_make(USER),
                    vec![Coin {
                        denom: NATIVE_DENOM.to_string(),
                        amount: Uint128::new(1),
                    }],
                )
                .unwrap();
        })
    }

    fn proper_instantiate() -> (App, CwTemplateContract) {
        let mut app = mock_app();
        let cw_template_id = app.store_code(contract_template());

        let user = app.api().addr_make(USER);
        assert_eq!(
            app.wrap().query_balance(user, NATIVE_DENOM).unwrap().amount,
            Uint128::new(1)
        );

        let msg = InstantiateMsg { count: 1i32 };
        let cw_template_contract_addr = app
            .instantiate_contract(
                cw_template_id,
                Addr::unchecked(ADMIN),
                &msg,
                &[],
                "test",
                None,
            )
            .unwrap();

        let cw_template_contract = CwTemplateContract(cw_template_contract_addr);

        (app, cw_template_contract)
    }

    mod count {
        use super::*;
        use crate::msg::ExecuteMsg;

        #[test]
        fn count() {
            let (mut app, cw_template_contract) = proper_instantiate();

            let msg = ExecuteMsg::Increment {};
            let cosmos_msg = cw_template_contract.call(msg).unwrap();
            app.execute(Addr::unchecked(USER), cosmos_msg).unwrap();
        }
    }
    fn counter_contract() -> Box<dyn Contract<Empty>> {
        Box::new(ContractWrapper::new_with_empty(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        ))
    }
    #[test]
    fn incrementing_should_work() {
        let mut app = App::default();

        let code_id = app.store_code(counter_contract());

        let owner = "owner".into_addr();

        let contract_addr = app
            .instantiate_contract(
                code_id,
                owner.clone(),
                &CounterInitMsg { count: 0 },
                &[],
                "counter-contract",
                None,
            )
            .unwrap();

        app.execute_contract(
            owner,
            contract_addr.clone(),
            &CounterExecMsg::Increment {},
            &[],
        )
        .unwrap();

        let res: CounterResponse = app
            .wrap()
            .query_wasm_smart(contract_addr, &CounterQueryMsg::GetCount {})
            .unwrap();

        println!("count: {:?}", res.count);
        assert_eq!(1, res.count);
    }

    //Testing related to the staking contract

    #[test]
    fn test_delegation_undelegation_cycle() {
        // Setup test accounts
        let delegator = "delegator".into_addr();
        let validator = "validator".into_addr();

        // Initial balance for delegator
        let initial_balance = coins(1000, "TOKEN");

        // Create app with staking module enabled
        let mut app = App::new(|router, api, storage| {
            // Initialize delegator with funds
            router
                .bank
                .init_balance(storage, &delegator, initial_balance.clone())
                .unwrap();

            // Setup staking module with validator
            let block = BlockInfo {
                height: 1,
                time: cosmwasm_std::Timestamp::from_seconds(1),
                chain_id: "test".to_string(),
            };
            let validator = Validator::new(
                validator.to_string(),
                Decimal::percent(20),
                Decimal::percent(20),
                Decimal::percent(1),
            );

            router
                .staking
                .add_validator(api, storage, &block, validator)
                .unwrap();
        });

        // Get the validator operator object

        let validator_operator: Option<Validator> =
            app.wrap().query_validator(validator.as_str()).unwrap();

        // Amount to delegate
        let delegation_amount = coin(100, "TOKEN");

        // Delegate tokens
        app.execute(
            delegator.clone(),
            CosmosMsg::Staking(StakingMsg::Delegate {
                validator: validator.to_string(),
                amount: delegation_amount.clone(),
            }),
        )
        .unwrap();

        // Verify the balance of the delegator
        let delegator_balance = app
            .wrap()
            .query_balance(delegator.as_str(), "TOKEN")
            .unwrap();
        assert_eq!(delegator_balance.amount.u128(), 900); // 1000 - 100 = 900

        // Check the amount staked by the delegator to the validator
        let validator_balance = app
            .wrap()
            .query_all_delegations(delegator.as_str())
            .unwrap();
        assert_eq!(validator_balance[0].amount, delegation_amount);

        // Check the amount staked to the validator (aka the validator delegation)
        // validator_address.unwrap();
        // ????
        // Undelegate tokens
        app.execute(
            delegator.clone(),
            CosmosMsg::<Empty>::Staking(StakingMsg::Undelegate {
                validator: validator.to_string(),
                amount: delegation_amount.clone(),
            }),
        )
        .unwrap();

        // Delegator balance should still be 900 as undelegation is in progress
        let delegator_balance = app
            .wrap()
            .query_balance(delegator.as_str(), "TOKEN")
            .unwrap();
        assert_eq!(delegator_balance.amount.u128(), 900);

        println!("delegator_balance: {:?}", delegator_balance);

        // Advance block time to simulate undelegation period (typically 21 days in Cosmos)
        app.update_block(|block| {
            block.time = block.time.plus_days(21);
            block.height += 21 * 100; // Assuming ~100 blocks per day
        });

        // After undelegation period, funds should be returned
        // Note: In cw-multi-test, we need to trigger a block update or process pending operations
        app.update_block(|block| {
            block.height += 1;
        });

        // Verify funds are returned to delegator
        let delegator_balance = app
            .wrap()
            .query_balance(delegator.as_str(), "TOKEN")
            .unwrap();
        assert_eq!(delegator_balance.amount.u128(), 1000); // Should be back to initial balance

        //Add an other delegator so we're not out of bounds

        // Check the amount staked by the delegator to the validator
        let validator_balance = app
            .wrap()
            .query_all_delegations(delegator.as_str())
            .unwrap();

        // Assert that we get an error when trying to access the first delegation
        // since the delegator has undelegated all their tokens
        assert!(validator_balance.is_empty());
    }
}
