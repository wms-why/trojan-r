mod client;
mod forward;
mod global_config;
mod server;

use std::{
    fs::File,
    io::{self, Read},
    sync::Arc,
};

use serde::Deserialize;
use tokio::io::{split, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{
    error::Error,
    protocol::{
        AcceptResult, ProxyAcceptor, ProxyConnector, ProxyTcpStream, ProxyUdpStream, UdpRead,
        UdpWrite,
    },
};

use self::{client::ClientProxy, forward::ForwardProxy, server::ServerProxy};

const RELAY_BUFFER_SIZE: usize = 0x4000;

async fn copy_udp<R: UdpRead, W: UdpWrite>(r: &mut R, w: &mut W) -> io::Result<()> {
    let mut buf = [0u8; RELAY_BUFFER_SIZE];
    loop {
        let (len, addr) = r.read_from(&mut buf).await?;
        log::debug!("udp packet addr={} len={}", addr, len);
        if len == 0 {
            break;
        }
        w.write_to(&buf[..len], &addr).await?;
    }
    Ok(())
}

async fn copy_tcp<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    r: &mut R,
    w: &mut W,
) -> io::Result<()> {
    let mut buf = [0u8; RELAY_BUFFER_SIZE];
    loop {
        let len = r.read(&mut buf).await?;
        if len == 0 {
            break;
        }
        w.write(&buf[..len]).await?;
        w.flush().await?;
    }
    Ok(())
}

pub async fn relay_udp<T: ProxyUdpStream, U: ProxyUdpStream>(a: T, b: U) {
    let (mut a_rx, mut a_tx) = a.split();
    let (mut b_rx, mut b_tx) = b.split();
    let t1 = copy_udp(&mut a_rx, &mut b_tx);
    let t2 = copy_udp(&mut b_rx, &mut a_tx);
    let e = tokio::select! {
        e = t1 => {e}
        e = t2 => {e}
    };
    if let Err(e) = e {
        log::debug!("udp_relay err: {}", e)
    }
    let _ = T::reunite(a_rx, a_tx).close().await;
    let _ = U::reunite(b_rx, b_tx).close().await;
    log::info!("udp session ends");
}

pub async fn relay_tcp<T: ProxyTcpStream, U: ProxyTcpStream>(a: T, b: U) {
    let (mut a_rx, mut a_tx) = split(a);
    let (mut b_rx, mut b_tx) = split(b);
    let t1 = copy_tcp(&mut a_rx, &mut b_tx);
    let t2 = copy_tcp(&mut b_rx, &mut a_tx);
    let e = tokio::select! {
        e = t1 => {e}
        e = t2 => {e}
    };
    if let Err(e) = e {
        log::debug!("relay_tcp err: {}", e)
    }
    let mut a = a_rx.unsplit(a_tx);
    let mut b = b_rx.unsplit(b_tx);
    let _ = a.shutdown().await;
    let _ = b.shutdown().await;
    log::info!("tcp session ends");
}

async fn run_proxy<I: ProxyAcceptor, O: ProxyConnector + 'static>(
    acceptor: I,
    connector: O,
) -> io::Result<()> {
    let connector = Arc::new(connector);
    loop {
        match acceptor.accept().await {
            Ok(AcceptResult::Tcp((inbound, addr))) => {
                let connector = connector.clone();
                tokio::spawn(async move {
                    match connector.connect_tcp(&addr).await {
                        Ok(outbound) => {
                            log::info!("relaying tcp stream to {}", addr);
                            relay_tcp(inbound, outbound).await;
                        }
                        Err(e) => {
                            log::error!("failed to relay tcp stream to {}: {}", addr, e);
                        }
                    }
                });
            }
            Ok(AcceptResult::Udp(inbound)) => {
                let connector = connector.clone();
                tokio::spawn(async move {
                    match connector.connect_udp().await {
                        Ok(outbound) => {
                            log::info!("relaying udp stream..");
                            relay_udp(inbound, outbound).await;
                        }
                        Err(e) => {
                            log::error!("failed to relay tcp stream: {}", e.to_string());
                        }
                    }
                });
            }
            Err(e) => {
                log::error!("accept failed: {}", e);
            }
        }
    }
}

pub async fn launch_from_config_filename(filename: String) -> io::Result<()> {
    let mut file = File::open(filename)?;
    let mut config_string = String::new();
    file.read_to_string(&mut config_string)?;
    launch_from_config_string(config_string).await
}

pub(crate) trait Proxy {
    async fn start() -> io::Result<()>;
}

enum ProxyMode<T> {
    Server(T),
    Client(T),
    Forward(T),
}

impl<T: Proxy> From<&str> for ProxyMode<T> {
    fn from(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "server" => ProxyMode::Server(ServerProxy {}),
            "client" => ProxyMode::Client(ClientProxy {}),
            "forward" => ProxyMode::Forward(ForwardProxy {}),
            _ => panic!("invalid mode: {}", value),
        }
    }
}

impl<T: Proxy> ProxyMode<T> {
    async fn init_proxy(value: T) -> io::Result<()> {
        value.start()?;
        Ok(())
    }
}

#[derive(Deserialize)]
struct BaseConfig {
    pub mode: String,
}

pub async fn launch_from_config_string(config_string: String) -> io::Result<()> {
    let config: BaseConfig = toml::from_str(&config_string)?;

    let mode: ProxyMode = ProxyMode::from(config.mode.as_str());

    match mode {
        ProxyMode::Server(_) => {
            ServerProxy::start()?;
        }
        ProxyMode::Client(_) => {
            ClientProxy::start()?;
        }
        ProxyMode::Forward(_) => {
            ForwardProxy::start()?;
        }
        _ => {
            log::error!("invalid mode: {}", config.mode.as_str());
            return Err(Error::new("invalid mode"));
        }
    }
    Ok(())
}
