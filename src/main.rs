mod client;
mod server;
mod com;

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    
    let configs = com::Config::from_file("config.json").await;
    let configc = configs.clone();
    let s = tokio::spawn(async {
        if let Some(config) = configs {
            if let Some(sconfig) = config.server {
                server::run(sconfig).await;
            }
        }
    });
    let c = tokio::spawn(async {
        if let Some(config) = configc {
            if let Some(cconfig) = config.client {
                client::run(cconfig).await;
            }
        }
    });
    s.await.unwrap();
    c.await.unwrap();
}
