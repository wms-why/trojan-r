use std::io;

use serde::Deserialize;

use crate::{
    error::Error,
    protocol::{
        direct::connector::DirectConnector,
        mux::acceptor::{MuxAcceptor, MuxAcceptorConfig},
        plaintext::acceptor::{PlaintextAcceptor, PlaintextAcceptorConfig},
        tls::acceptor::{TrojanTlsAcceptor, TrojanTlsAcceptorConfig},
        trojan::acceptor::{TrojanAcceptor, TrojanAcceptorConfig},
        websocket::acceptor::{WebSocketAcceptor, WebSocketAcceptorConfig},
    },
    proxy::run_proxy,
};

use super::Proxy;

#[derive(Deserialize)]
pub struct ServerConfig {
    trojan: TrojanAcceptorConfig,
    tls: Option<TrojanTlsAcceptorConfig>,
    plaintext: Option<PlaintextAcceptorConfig>,
    websocket: Option<WebSocketAcceptorConfig>,
    mux: Option<MuxAcceptorConfig>,
}

pub struct ServerProxy {}

impl Proxy for ServerProxy {
    async fn start(config_string: String) -> io::Result<()> {
        log::debug!("server mode");
        let config: ServerConfig = toml::from_str(&config_string)?;
        
        let direct_connector = DirectConnector {};
        if config.tls.is_none() {
            if config.plaintext.is_none() {
                return Err(Error::new("plaintext/tls section not found").into());
            }
            let direct_acceptor = PlaintextAcceptor::new(&config.plaintext.unwrap()).await?;
            if config.websocket.is_none() {
                let trojan_acceptor = TrojanAcceptor::new(&config.trojan, direct_acceptor)?;
                if config.mux.is_none() {
                    run_proxy(trojan_acceptor, direct_connector).await?;
                } else {
                    let mux_acceptor = MuxAcceptor::new(trojan_acceptor, &config.mux.unwrap())?;
                    run_proxy(mux_acceptor, direct_connector).await?;
                }
            } else {
                let ws_acceptor =
                    WebSocketAcceptor::new(&config.websocket.unwrap(), direct_acceptor)?;
                let trojan_acceptor = TrojanAcceptor::new(&config.trojan, ws_acceptor)?;
                if config.mux.is_none() {
                    run_proxy(trojan_acceptor, direct_connector).await?;
                } else {
                    let mux_acceptor = MuxAcceptor::new(trojan_acceptor, &config.mux.unwrap())?;
                    run_proxy(mux_acceptor, direct_connector).await?;
                }
            }
        } else {
            let tls_acceptor = TrojanTlsAcceptor::new(&config.tls.unwrap()).await?;
            if config.websocket.is_none() {
                let trojan_acceptor = TrojanAcceptor::new(&config.trojan, tls_acceptor)?;
                if config.mux.is_none() {
                    run_proxy(trojan_acceptor, direct_connector).await?;
                } else {
                    let mux_acceptor = MuxAcceptor::new(trojan_acceptor, &config.mux.unwrap())?;
                    run_proxy(mux_acceptor, direct_connector).await?;
                }
            } else {
                let ws_acceptor = WebSocketAcceptor::new(&config.websocket.unwrap(), tls_acceptor)?;
                let trojan_acceptor = TrojanAcceptor::new(&config.trojan, ws_acceptor)?;
                if config.mux.is_none() {
                    run_proxy(trojan_acceptor, direct_connector).await?;
                } else {
                    let mux_acceptor = MuxAcceptor::new(trojan_acceptor, &config.mux.unwrap())?;
                    run_proxy(mux_acceptor, direct_connector).await?;
                }
            }
        }
    }
}
