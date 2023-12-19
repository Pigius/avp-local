use actix_web::{get, web, App, HttpResponse, HttpServer, Responder, Result};
use serde::{Serialize, Deserialize};
use avp_local_agent::public::policy_set_provider::PolicySetProvider;
use avp_local_agent::public::entity_provider::EntityProvider;
use avp_local_agent::public::client::verified_permissions_default_credentials;
use log::{info, error};
use std::sync::Arc;
use aws_sdk_verifiedpermissions::config::Region;

#[derive(Serialize)]
pub struct Response {
    pub message: String,
}

#[derive(Deserialize, Serialize)]
pub struct EvaluatePayload {
    policies: Vec<String>,
    resources: Vec<String>,
    action: String,
    principal: String,
    context: Option<serde_json::Value>,
}

#[get("/health")]
async fn healthcheck() -> impl Responder {
    let response = Response {
        message: "Everything is working fine".to_string(),
    };
    HttpResponse::Ok().json(response)
}

async fn not_found() -> Result<HttpResponse> {
    let response = Response {
        message: "Resource not found".to_string(),
    };
    Ok(HttpResponse::NotFound().json(response))
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Hello");

    let (policy_set_provider, entity_provider) = tokio::task::spawn_blocking(|| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(init_avp_components())
    }).await.unwrap();

    let policy_set_provider = Arc::new(policy_set_provider);
    let entity_provider = Arc::new(entity_provider);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(policy_set_provider.clone()))
            .app_data(web::Data::new(entity_provider.clone()))
            .service(healthcheck)
            .default_service(web::route().to(not_found))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

async fn init_avp_components() -> (PolicySetProvider, EntityProvider) {
    info!("Initializing AVP components");

    let client = verified_permissions_default_credentials(Region::new("eu-east-1")).await;
    info!("AVP client initialized successfully");

    let policy_store_id = "bla".to_string();
    info!("Using policy store ID: {}", policy_store_id);

    let policy_set_provider = match PolicySetProvider::from_client(policy_store_id.clone(), client.clone()) {
        Ok(provider) => {
            info!("Policy set provider initialized successfully");
            provider
        },
        Err(e) => {
            error!("Error initializing policy set provider: {:?}", e);
            panic!("Failed to initialize policy set provider"); // or handle the error appropriately
        }
    };

    let entity_provider = match EntityProvider::from_client(policy_store_id, client) {
        Ok(provider) => {
            info!("Entity provider initialized successfully");
            provider
        },
        Err(e) => {
            error!("Error initializing entity provider: {:?}", e);
            panic!("Failed to initialize entity provider"); // or handle the error appropriately
        }
    };

    (policy_set_provider, entity_provider)
}

