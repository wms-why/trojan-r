
#[derive(Deserialize)]
pub struct ForwardConfig {
    dokodemo: DokodemoAcceptorConfig,
    trojan: TrojanConnectorConfig,
    tls: TrojanTlsConnectorConfig,
    websocket: Option<WebSocketConnectorConfig>,
    mux: Option<MuxConnectorConfig>,
}
pub struct ForwardProxy {}

impl Proxy for ForwardProxy {
    fn start() {
        log::debug!("forward mode");
            let config: ForwardConfig = toml::from_str(&config_string)?;
            let dokodemo_acceptor = DokodemoAcceptor::new(&config.dokodemo).await?;
            let tls_connector = TrojanTlsConnector::new(&config.tls)?;
            if config.websocket.is_none() {
                let trojan_connector = TrojanConnector::new(&config.trojan, tls_connector)?;
                if config.mux.is_none() {
                    run_proxy(dokodemo_acceptor, trojan_connector).await?;
                } else {
                    let mux_connector =
                        MuxConnector::new(&config.mux.unwrap(), trojan_connector).unwrap();
                    run_proxy(dokodemo_acceptor, mux_connector).await?;
                }
            } else {
                let ws_connector =
                    WebSocketConnector::new(&config.websocket.unwrap(), tls_connector)?;
                let trojan_connector = TrojanConnector::new(&config.trojan, ws_connector)?;
                if config.mux.is_none() {
                    run_proxy(dokodemo_acceptor, trojan_connector).await?;
                } else {
                    let mux_connector =
                        MuxConnector::new(&config.mux.unwrap(), trojan_connector).unwrap();
                    run_proxy(dokodemo_acceptor, mux_connector).await?;
                }
            }
    }
}