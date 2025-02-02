use sncast_std::{
    declare, deploy, invoke, call, DeclareResult, DeployResult, InvokeResult, CallResult
};

fn second_contract() {
    let declare_result = declare('Mapa2', Option::None);
    let deploy_result = deploy(
        declare_result.class_hash, ArrayTrait::new(), Option::None, false, Option::None
    );
    assert(deploy_result.transaction_hash != 0, deploy_result.transaction_hash);

    let invoke_result = invoke(
        deploy_result.contract_address, 'put', array![0x1, 0x3], Option::None
    );
    assert(invoke_result.transaction_hash != 0, invoke_result.transaction_hash);

    let call_result = call(deploy_result.contract_address, 'get', array![0x1]);
    assert(call_result.data == array![0x3], *call_result.data.at(0));
}

fn main() {
    let max_fee = 99999999999999999;
    let salt = 0x3;

    let declare_result = declare('Mapa', Option::Some(max_fee));

    let class_hash = declare_result.class_hash;
    let deploy_result = deploy(
        class_hash, ArrayTrait::new(), Option::Some(salt), true, Option::Some(max_fee)
    );
    assert(deploy_result.transaction_hash != 0, deploy_result.transaction_hash);

    let invoke_result = invoke(
        deploy_result.contract_address, 'put', array![0x1, 0x2], Option::Some(max_fee)
    );
    assert(invoke_result.transaction_hash != 0, invoke_result.transaction_hash);

    let call_result = call(deploy_result.contract_address, 'get', array![0x1]);
    assert(call_result.data == array![0x2], *call_result.data.at(0));

    second_contract();
}
