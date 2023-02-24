use std::{
    fs::File,
    io::BufReader,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
};

use quinn::{Endpoint, RecvStream, SendStream, ServerConfig};
use tokio::{io::AsyncWriteExt, net::TcpStream};

use crate::com::Server;

pub async fn run(config: Server) {
    let file = File::open(Path::new(&config.public_key_file))
        .expect(format!("cannot open {}", &config.public_key_file).as_str());
    let mut br = BufReader::new(file);
    let cetrs = rustls_pemfile::certs(&mut br).unwrap();

    let filek = File::open(Path::new(&config.private_key_file))
        .expect(format!("cannot open {}", &config.private_key_file).as_str());
    let mut brk = BufReader::new(filek);
    let keys = rustls_pemfile::pkcs8_private_keys(&mut brk).unwrap();

    let certificate = rustls::Certificate(cetrs[0].clone());
    let private_key = rustls::PrivateKey(keys[0].clone());

    let cert_chain = vec![certificate];

    let server_config = ServerConfig::with_single_cert(cert_chain, private_key).unwrap();

    let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.port);
    let endpoint = Endpoint::server(server_config, bind_addr).unwrap();

    log::info!("Server start in {}", config.port);

    while let Some(income_conn) = endpoint.accept().await {
        if let Ok(new_conn) = income_conn.await {
            tokio::spawn(async move {
                loop {
                    match new_conn.accept_bi().await {
                        Ok((wstream, rstream)) => {
                            tokio::spawn(async move {
                                handle(wstream, rstream).await;
                            });
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
            });
        }
    }
}

static mut COUNT: AtomicUsize = AtomicUsize::new(0);

async fn handle(mut wstream: SendStream, mut rstream: RecvStream) {
    let mut forward = [0u8; 6];
    if let Err(_) = rstream.read_exact(&mut forward).await {
        return;
    }
    let port = ((forward[4] as u16) << 8) | (forward[5] as u16);
    let dst = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(
            forward[0], forward[1], forward[2], forward[3],
        )),
        port,
    );
    if let Ok(mut tcp) = TcpStream::connect(dst).await {
        unsafe {
            let cur = COUNT.fetch_add(1, Ordering::Relaxed);
            log::info!(
                "forward start dst={}.{}.{}.{}:{} count={}",
                forward[0],
                forward[1],
                forward[2],
                forward[3],
                port,
                cur + 1
            );
        }
        let (mut r, mut w) = tcp.split();
        let t1 = async {
            let _ = tokio::io::copy(&mut r, &mut wstream).await;
            wstream.shutdown().await
        };
        let t2 = async {
            let _ = tokio::io::copy(&mut rstream, &mut w).await;
            w.shutdown().await
        };
        let _ = tokio::join!(t1, t2);
        unsafe {
            let cur = COUNT.fetch_sub(1, Ordering::Relaxed);
            log::info!(
                "forward end dst={}.{}.{}.{}:{} count={}",
                forward[0],
                forward[1],
                forward[2],
                forward[3],
                port,
                cur - 1
            );
        }
    }
}
