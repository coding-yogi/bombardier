use log::{error, info, warn};
use reqwest::{
    Client, 
    Request,
    Response, 
    Certificate, 
    Identity
};
use tokio::fs;

use std::{
    error::Error,
    time,
};

use crate::{cmd};

pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub async fn new(config: &cmd::ExecConfig) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let client = get_async_client(config).await?;

        Ok(HttpClient {client})
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }
}

async fn get_certificate(path: &str)  -> Result<Certificate, Box<dyn Error + Send + Sync>> {
    info!("Getting certificate file from path {}", path);
    let cert = match fs::read(path).await {
        Ok(cert) => cert,
        Err(err) => {
            error!("Reading certificate file failed: {}", err);
            return Err(err.into())
        }
    };
    
    if path.to_lowercase().ends_with(cmd::DER_EXT) {
        return Ok(Certificate::from_der(&cert)?)
    } else if path.to_lowercase().ends_with(cmd::PEM_EXT) {
        return Ok(Certificate::from_pem(&cert)?)
    }

    Err("Certificate should be in .pem or .der format".into())
}

async fn get_identity(path: &str, password: &str) -> Result<Identity, Box<dyn Error + Send + Sync>> {
    info!("Getting keystore file from path {}", path);
    let ks = match fs::read(path).await {
        Ok(ks) => ks,
        Err(err) => {
            error!("Reading keystore file failed: {}", err);
            return Err(err.into())
        }
    };


    if path.to_lowercase().ends_with(cmd::P12_EXT) || path.to_lowercase().ends_with(cmd::PFX_EXT) {
        return Ok(Identity::from_pkcs12_der(&ks, password)?)
    }

    Err("Keystore should be in .p12 or .pfx format".into())
}

async fn get_async_client(config: &cmd::ExecConfig)  -> Result<Client, Box<dyn Error + Send + Sync>> {
    let mut client_builder = Client::builder()
        .user_agent("bombardier")
        .use_native_tls();

    if config.handle_cookies {
        info!("Enabling cookie store");
        client_builder = client_builder.cookie_store(true);
    }

    if config.ssl.ignore_ssl {
        warn!("SSL validation has been disabled, this is dangerous as all invalid certs would be accepted");
        client_builder = client_builder.danger_accept_invalid_certs(true);
    } else {
        if config.ssl.accept_invalid_hostnames {
            warn!("SSL hostname validation has been disabled, this is dangerous as all certs with non matching hostnames would be accepted");
            client_builder = client_builder.danger_accept_invalid_hostnames(true);
        }
    
        if config.ssl.certificate != "" {
            let cert = get_certificate(&config.ssl.certificate).await?;
            info!("Adding new trusted certificate {}", &config.ssl.certificate);
            client_builder = client_builder.add_root_certificate(cert);
        }

        if config.ssl.keystore != "" {
            let ks = get_identity(&config.ssl.keystore, &config.ssl.keystore_password).await?;
            info!("Adding new keystore {}", &config.ssl.keystore);
            client_builder = client_builder.identity(ks);
        }
    }

    Ok(client_builder.build()?)
}

impl HttpClient {
    pub async fn execute(&self, request: Request) -> Result<(Response, u128), Box<dyn Error + Send + Sync>>  {  
        //Initialising timestamps
        let start_time = time::Instant::now();
        let resp = self.client.execute(request).await?;
        let end_time = start_time.elapsed().as_millis();
       
        Ok((resp, end_time))
    }
}

impl HttpClient {
    pub fn get_default_sync_client() -> Result<HttpClient, reqwest::Error> {
        let client = Client::builder()
            .user_agent("bombardier")
            .build()?;

        Ok(HttpClient {client})
    } 
}