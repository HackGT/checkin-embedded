use std::ops::{ Deref, DerefMut };
use std::io::Read;
use rocket::request::{ self, Request, FromRequest };
use rocket::{ Data, Outcome, State };
use rocket::http::{ Status, ContentType };
use rocket::data::{ self, FromDataSimple };
use rocket_contrib::json::{ Json, JsonValue };
use serde::Deserialize;
use serde::de::DeserializeOwned;
use ed25519_dalek::{ PublicKey, Signature };
use wither::model::Model;
use hackgt_nfc::api::CheckinAPI;
use crate::DB;
use crate::models::Device;
use crate::auth::AuthenticatedUser;

pub struct IP(String);

impl IP {
    fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for IP {
    type Error = !;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        // Gets IP from either X-Real-IP (if behind proxy) or directly from the connection
        let ip = request.client_ip()
            .map(|ip| ip.to_string())
            .unwrap_or(String::from("Unknown IP"));
        Outcome::Success(IP(ip))
    }
}

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
pub fn initialize(request: SignedRequest<InitializeRequest>, db: State<DB>, ip: IP) -> Result<JsonValue, mongodb::error::Error> {
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
            device.update(
                db.clone(),
                None,
                doc! { "$set": {
                    "ip_address": ip.as_str(),
                } },
                None
            )?;
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
                ip_address: ip.as_str().to_owned(),

                authorized: false,
                status_set_by: None,
                pending: true,
                credentials_created: false,

                current_tag: None,
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
                        None,
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

#[get("/tag?<username>")]
pub fn get_tag(username: Option<String>, db: State<DB>, checkin_api: State<CheckinAPI>) -> Result<JsonValue, mongodb::error::Error> {
    let mut tags = checkin_api.get_tags_names(false).unwrap_or(Vec::new());
    tags.sort();

    let current_tag: Option<String> = match username {
        Some(username) => Device::find_one(db.clone(), Some(doc! { "username": &username }), None)?.and_then(|device| device.current_tag),
        None => None
    };

    Ok(json!({
        "current": current_tag,
        "all": tags,
    }))
}

// Device actions called by JS in web UI
#[derive(Deserialize)]
pub struct DeviceButtonAction {
    username: String,
}

#[post("/device/authorize", format = "json", data = "<request>")]
pub fn authorize_device(user: AuthenticatedUser, request: Json<DeviceButtonAction>, db: State<DB>) -> Result<JsonValue, mongodb::error::Error> {
    let response = match Device::find_one(db.clone(), Some(doc! { "username": &request.username }), None)? {
        Some(device) => {
            device.update(
                db.clone(),
                None,
                doc! { "$set": {
                    "authorized": true,
                    "status_set_by": &user.username,
                    "pending": false,
                } },
                None
            )?;
            json!({
                "success": true,
            })
        },
        None => {
            json!({
                "success": false,
                "error": "Device not found",
            })
        }
    };
    Ok(response)
}

#[post("/device/reject", format = "json", data = "<request>")]
pub fn reject_device(user: AuthenticatedUser, request: Json<DeviceButtonAction>, db: State<DB>) -> Result<JsonValue, mongodb::error::Error> {
    let response = match Device::find_one(db.clone(), Some(doc! { "username": &request.username }), None)? {
        Some(device) => {
            device.update(
                db.clone(),
                None,
                doc! { "$set": {
                    "authorized": false,
                    "status_set_by": &user.username,
                    "pending": false,
                } },
                None
            )?;
            json!({
                "success": true,
            })
        },
        None => {
            json!({
                "success": false,
                "error": "Device not found",
            })
        }
    };
    Ok(response)
}

#[post("/device/force-renew", format = "json", data = "<request>")]
pub fn force_renew_device(request: Json<DeviceButtonAction>, db: State<DB>, checkin_api: State<CheckinAPI>) -> Result<JsonValue, mongodb::error::Error> {
    let response = match Device::find_one(db.clone(), Some(doc! { "username": &request.username }), None)? {
        Some(device) => {
            match checkin_api.delete_user(&request.username) {
                Ok(_) => {
                    device.update(
                        db.clone(),
                        None,
                        doc! { "$set": {
                            "credentials_created": false,
                        } },
                        None
                    )?;
                    json!({
                        "success": true,
                    })
                },
                Err(err) => {
                    json!({
                        "success": false,
                        "error": "Failed to delete user account",
                        "details": format!("{:?}", err),
                    })
                }
            }
        },
        None => {
            json!({
                "success": false,
                "error": "Device not found",
            })
        }
    };
    Ok(response)
}

#[post("/device/delete", format = "json", data = "<request>")]
pub fn delete_device(request: Json<DeviceButtonAction>, db: State<DB>, checkin_api: State<CheckinAPI>) -> Result<JsonValue, mongodb::error::Error> {
    let response = match Device::find_one(db.clone(), Some(doc! { "username": &request.username }), None)? {
        Some(device) => {
            if device.credentials_created {
                // Delete this device's checkin2 account if one exists
                if let Err(err) = checkin_api.delete_user(&request.username) {
                    return Ok(json!({
                        "success": false,
                        "error": "Failed to delete device's checkin2 account",
                        "details": format!("{:?}", err),
                    }));
                }
            }
            device.delete(db.clone())?;
            json!({
                "success": true,
            })
        },
        None => {
            json!({
                "success": false,
                "error": "Device not found",
            })
        }
    };
    Ok(response)
}

#[derive(Deserialize)]
pub struct DeviceTagAction {
    username: String,
    tag: String,
}
#[post("/device/set-tag", format = "json", data = "<request>")]
pub fn set_tag(request: Json<DeviceTagAction>, db: State<DB>) -> Result<JsonValue, mongodb::error::Error> {
    let response = match Device::find_one(db.clone(), Some(doc! { "username": &request.username }), None)? {
        Some(device) => {
            device.update(
                db.clone(),
                None,
                doc! { "$set": {
                    "current_tag": request.tag.clone(),
                } },
                None
            )?;
            json!({
                "success": true,
            })
        },
        None => {
            json!({
                "success": false,
                "error": "Device not found",
            })
        }
    };
    Ok(response)
}

#[derive(Deserialize)]
pub struct DeviceRenameAction {
    username: String,
    name: String,
}
#[post("/device/rename", format = "json", data = "<request>")]
pub fn rename_device(request: Json<DeviceRenameAction>, db: State<DB>) -> Result<JsonValue, mongodb::error::Error> {
    let response = match Device::find_one(db.clone(), Some(doc! { "username": &request.username }), None)? {
        Some(device) => {
            device.update(
                db.clone(),
                None,
                doc! { "$set": {
                    "friendly_name": request.name.clone(),
                } },
                None
            )?;
            json!({
                "success": true,
            })
        },
        None => {
            json!({
                "success": false,
                "error": "Device not found",
            })
        }
    };
    Ok(response)
}
