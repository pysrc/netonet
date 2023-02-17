use serde::{Serialize, Deserialize};
use tokio::{fs::File, io::AsyncReadExt};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Iomap {
    pub inner: u16,
    pub outer: [u16;5],
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Client {
    pub server: String,
    #[serde(rename = "public-key-file")]
    pub public_key_file: String,
    pub map: Vec<Iomap>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub port: u16,
    #[serde(rename = "private-key-file")]
    pub private_key_file: String,
    #[serde(rename = "public-key-file")]
    pub public_key_file: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub server: Option<Server>,
    pub client: Option<Client>,
}

impl Config {
    pub async fn from_str(js: &str) -> Option<Config> {
        let res: Option<Config> = match serde_json::from_str(js) {
            Ok(cfg) => Some(cfg),
            Err(e) => {
                panic!("error {}", e);
            }
        };
        return res;
    }

    pub async fn from_file(filename: &str) -> Option<Config> {
        let f = File::open(filename).await;
        match f {
            Ok(mut file) => {
                let mut c = String::new();
                file.read_to_string(&mut c).await.unwrap();
                Config::from_str(&c).await
            }
            Err(e) => {
                panic!("error {}", e)
            }
        }
    }
}