#[cfg(test)]
mod tests {
    use crate::helpers::CwTemplateContract;
    use crate::msg::InstantiateMsg;
    use crate::msg::{
        ExecuteMsg as CounterExecMsg, GetCountResponse as CounterResponse,
        InstantiateMsg as CounterInitMsg, QueryMsg as CounterQueryMsg,
    };
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::{Addr, Coin, Empty, Uint128};
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
}
