use std::{
    fs::File,
    io::BufReader,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
    str::FromStr,
};

use quinn::{ClientConfig, Endpoint};
use tokio::{
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

use crate::com::{Client, Iomap};

pub async fn run(config: Client) {
    let file = File::open(Path::new(&config.public_key_file))
        .expect(format!("cannot open {}", &config.public_key_file).as_str());
    let mut br = BufReader::new(file);
    let cetrs = rustls_pemfile::certs(&mut br).unwrap();

    let certificate = rustls::Certificate(cetrs[0].clone());
    let mut certs = rustls::RootCertStore::empty();
    certs.add(&certificate).unwrap();

    let client_config = ClientConfig::with_root_certificates(certs);

    let endpoint = {
        let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
        let mut endpoint = Endpoint::client(bind_addr).unwrap();
        endpoint.set_default_client_config(client_config);
        endpoint
    };
    let server_addr = SocketAddr::from_str(&config.server).unwrap();

    let mut ths = Vec::<JoinHandle<()>>::with_capacity(config.map.len());
    for m in config.map {
        let cfg = m.clone();
        let endpoint = endpoint.clone();
        let server_addr = server_addr.clone();
        let th = tokio::spawn(async move {
            handle(endpoint, server_addr, cfg).await;
        });
        ths.push(th);
    }
    for th in ths {
        th.await.unwrap();
    }
}

async fn handle(endpoint: Endpoint, server_addr: SocketAddr, cfg: Iomap) {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", cfg.inner))
        .await
        .unwrap();
    let mut dst = [0u8; 6];
    dst[0] = cfg.outer[0] as u8;
    dst[1] = cfg.outer[1] as u8;
    dst[2] = cfg.outer[2] as u8;
    dst[3] = cfg.outer[3] as u8;
    dst[4] = (cfg.outer[4] >> 8) as u8;
    dst[5] = cfg.outer[4] as u8;

    loop {
        let endpoint = endpoint.clone();
        let server_addr = server_addr.clone();
        let (tcpstream, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            trans(endpoint, server_addr, tcpstream, &dst).await;
        });
    }
}

async fn trans(endpoint: Endpoint, server_addr: SocketAddr, mut tcpstream: TcpStream, dst: &[u8]) {
    let new_conn = match endpoint.connect(server_addr, "netonet") {
        Ok(c) => match c.await {
            Ok(cc) => cc,
            Err(e) => {
                log::error!("Cannot connect remote {} ", e);
                return;
            }
        },
        Err(e) => {
            log::error!("Cannot connect remote {} ", e);
            return;
        }
    };

    let (mut r, mut w) = tcpstream.split();
    match new_conn.open_bi().await {
        Ok((mut wstream, mut rstream)) => {
            wstream.write(dst).await.unwrap();
            let f1 = async {
                let _ = tokio::io::copy(&mut r, &mut wstream).await;
            };
            let f2 = async {
                let _ = tokio::io::copy(&mut rstream, &mut w).await;
            };
            tokio::join!(f1, f2);
        }
        Err(_) => {}
    }
}
