use fuel_core::service::{
    Config,
    DbType,
    FuelService,
};
use fuel_core_client::client::FuelClient;
use fuel_core_interfaces::common::{
    fuel_tx,
    fuel_tx::{
        Address,
        AssetId,
    },
    fuel_vm::{
        consts::*,
        prelude::*,
    },
};
use tempfile::TempDir;

#[tokio::test]
async fn test_metrics_endpoint() {
    let mut config = Config::local_node();
    let tmp_dir = TempDir::new().unwrap();
    config.database_type = DbType::RocksDb;
    config.database_path = tmp_dir.path().to_path_buf();
    // setup server & client
    let srv = FuelService::new_node(config).await.unwrap();

    let client = FuelClient::from(srv.bound_address);
    let owner = Address::default();
    let asset_id = AssetId::new([1u8; 32]);
    // Should generate some database reads
    _ = client
        .balance(
            format!("{:#x}", owner).as_str(),
            Some(format!("{:#x}", asset_id).as_str()),
        )
        .await;
    let script = vec![
        Opcode::ADDI(0x10, REG_ZERO, 0xca),
        Opcode::ADDI(0x11, REG_ZERO, 0xba),
        Opcode::LOG(0x10, 0x11, REG_ZERO, REG_ZERO),
        Opcode::RET(REG_ONE),
    ];
    let script: Vec<u8> = script
        .iter()
        .flat_map(|op| u32::from(*op).to_be_bytes())
        .collect();

    client
        .submit_and_await_commit(
            &fuel_tx::Transaction::script(
                0,
                1000000,
                0,
                script,
                vec![],
                vec![],
                vec![],
                vec![],
            )
            .into(),
        )
        .await
        .unwrap();

    let resp = reqwest::get(format!("http://{}/metrics", srv.bound_address))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let categories = resp.split('\n').collect::<Vec<&str>>();

    srv.stop().await;

    // Gt check exists because testing can be weird with multiple instances running
    assert!(categories.len() == 16);

    for index in [2, 6, 7, 8, 11, 12, 13] {
        let value = categories[index].split(' ').collect::<Vec<&str>>()[1]
            .to_string()
            .parse::<f64>()
            .unwrap();

        assert!(value >= 0.0);
    }
}