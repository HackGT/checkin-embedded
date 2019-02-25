use std::ops::{ Deref, DerefMut };
use std::io::Read;
use std::net::SocketAddr;
use rocket::{ Request, Data, Outcome, State };
use rocket::http::{ Status, ContentType };
use rocket::data::{ self, FromDataSimple };
use rocket_contrib::json::JsonValue;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use ed25519_dalek::{ PublicKey, Signature };
use wither::model::Model;
use hackgt_nfc::api::CheckinAPI;
use crate::DB;
use crate::models::Device;

// Implement signature authentication for JSON bodies
#[derive(Debug)]
pub struct SignedRequest<T> {
    pub public_key: String,
    content: T,
}

impl<T> Deref for SignedRequest<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        &self.content
    }
}
impl<T> DerefMut for SignedRequest<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.content
    }
}

#[derive(Debug)]
pub enum SignedRequestError {
    Missing,
    Invalid,
    InvalidBody,
    Unauthorized,
}

impl<T: DeserializeOwned> FromDataSimple for SignedRequest<T> {
    type Error = SignedRequestError;

    fn from_data(request: &Request, data: Data) -> data::Outcome<Self, Self::Error> {
        let content_type = ContentType::new("application", "json");
        if request.content_type() != Some(&content_type) {
            return Outcome::Forward(data);
        }

        let auth: Option<&str> = request.headers().get("Authorization").next();
        match auth {
            Some(auth) => {
                let parts: Vec<_> = auth.split(" ").collect();
                if parts.get(0) != Some(&"ed25519") || parts.get(1).is_none() {
                    Outcome::Failure((Status::Unauthorized, SignedRequestError::Invalid))
                }
                else {
                    let parts: Vec<_> = parts.get(1).unwrap().split("/").collect();
                    let public_key = parts.get(0);
                    let signature = parts.get(1);
                    if public_key.is_none() || signature.is_none() {
                        return Outcome::Failure((Status::Unauthorized, SignedRequestError::Invalid));
                    }
                    // Get DB connection from Rocket request state
                    // let devices = request.guard::<State<DB>>().unwrap().collection("devices");
                    // let result = devices.find_one(Some(doc!{ "name": *public_key.unwrap() }), None);

                    // TODO: Check for unauthorized public keys
                    let raw_public_key = String::from(*public_key.unwrap());
                    let public_key = match hex::decode(public_key.unwrap()) {
                        Ok(key) => key,
                        Err(_) => return Outcome::Failure((Status::Unauthorized, SignedRequestError::Invalid)),
                    };
                    let public_key = match PublicKey::from_bytes(&public_key) {
                        Ok(key) => key,
                        Err(_) => return Outcome::Failure((Status::Unauthorized, SignedRequestError::Invalid)),
                    };
                    let signature = match hex::decode(signature.unwrap()) {
                        Ok(sig) => sig,
                        Err(_) => return Outcome::Failure((Status::Unauthorized, SignedRequestError::Invalid)),
                    };
                    let signature = match Signature::from_bytes(&signature) {
                        Ok(sig) => sig,
                        Err(_) => return Outcome::Failure((Status::Unauthorized, SignedRequestError::Invalid)),
                    };
                    // Verify signature
                    let mut body = Vec::new();
                    if let Err(_) = data.open().read_to_end(&mut body) {
                        return Outcome::Failure((Status::Unauthorized, SignedRequestError::InvalidBody));
                    }
                    if public_key.verify(&body, &signature).is_err() {
                        return Outcome::Failure((Status::Unauthorized, SignedRequestError::Invalid));
                    }
                    // Parse JSON
                    match serde_json::from_slice(&body) {
                        Ok(json) => Outcome::Success(SignedRequest { public_key: raw_public_key, content: json }),
                        Err(_) => Outcome::Failure((Status::Unauthorized, SignedRequestError::InvalidBody)),
                    }
                }
            },
            None => Outcome::Failure((Status::Unauthorized, SignedRequestError::Missing))
        }
    }
}

#[derive(Deserialize)]
pub struct InitializeRequest {
    username: String,
}

#[post("/initialize", format = "json", data = "<request>")]
pub fn initialize(request: SignedRequest<InitializeRequest>, db: State<DB>, remote_addr: SocketAddr) -> Result<JsonValue, mongodb::error::Error> {
    match Device::find_one(db.clone(), Some(doc! { "public_key": &request.public_key }), None)? {
        // Device already requested access, return status
        Some(device) => {
            let status: &'static str = if device.pending {
                "Pending"
            }
            else if device.authorized && device.credentials_created {
                "AuthorizedHasCredentials"
            }
            else if device.authorized && !device.credentials_created {
                "AuthorizedNoCredentials"
            }
            else {
                "Unauthorized"
            };
            Ok(json!({
                "status": status
            }))
        },
        // Device is brand new to us
        None => {
            let mut device = Device {
                id: None,

                public_key: request.public_key.clone(),
                username: request.username.clone(),
                friendly_name: String::from(&request.username[..16]),
                ip_address: remote_addr.ip().to_string(),

                authorized: false,
                pending: true,
                credentials_created: false,
            };
            device.save(db.clone(), None).unwrap();

            Ok(json!({
                "status": "Pending"
            }))
        }
    }
}

#[derive(Deserialize)]
pub struct CredentialsRequest {
    username: String,
    password: String,
}

#[post("/credentials", format = "json", data = "<request>")]
pub fn create_credentials(request: SignedRequest<CredentialsRequest>, db: State<DB>, checkin_api: State<CheckinAPI>) -> Result<JsonValue, mongodb::error::Error> {
    match Device::find_one(db.clone(), Some(doc! { "public_key": &request.public_key }), None)? {
        Some(device) => {
            if device.pending || !device.authorized {
                return Ok(json!({
                    "error": "Unauthorized or pending device"
                }));
            }
            let response = match checkin_api.add_user(&request.username, &request.password) {
                Ok(_) => {
                    device.update(
                        db.clone(),
                        Some(doc! { "public_key": &request.public_key }),
                        doc! { "$set": { "credentials_created": true } },
                        None
                    )?;
                    json!({
                        "success": true,
                    })
                },
                Err(err) => json!({
                    "error": "Failed to create user with credentials",
                    "details": format!("{:?}", err),
                }),
            };
            Ok(response)
        },
        _ => {
            Ok(json!({
                "error": "Unknown device"
            }))
        },
    }
}
