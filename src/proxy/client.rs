#[derive(Deserialize)]
pub struct ClientConfig {
    socks5: Socks5AcceptorConfig,
    trojan: TrojanConnectorConfig,
    tls: TrojanTlsConnectorConfig,
    websocket: Option<WebSocketConnectorConfig>,
    mux: Option<MuxConnectorConfig>,
}

pub struct ClientProxy {}
impl Proxy for ClientProxy {
    fn start() {
        log::debug!("client mode");
            let config: ClientConfig = toml::from_str(&config_string)?;
            let socks5_acceptor = Socks5Acceptor::new(&config.socks5).await?;
            let tls_connector = TrojanTlsConnector::new(&config.tls)?;
            if config.websocket.is_none() {
                let trojan_connector = TrojanConnector::new(&config.trojan, tls_connector)?;
                if config.mux.is_none() {
                    run_proxy(socks5_acceptor, trojan_connector).await?;
                } else {
                    let mux_connector =
                        MuxConnector::new(&config.mux.unwrap(), trojan_connector).unwrap();
                    run_proxy(socks5_acceptor, mux_connector).await?;
                }
            } else {
                let ws_connector =
                    WebSocketConnector::new(&config.websocket.unwrap(), tls_connector)?;
                let trojan_connector = TrojanConnector::new(&config.trojan, ws_connector)?;
                if config.mux.is_none() {
                    run_proxy(socks5_acceptor, trojan_connector).await?;
                } else {
                    let mux_connector =
                        MuxConnector::new(&config.mux.unwrap(), trojan_connector).unwrap();
                    run_proxy(socks5_acceptor, mux_connector).await?;
                }
            }
    }
}