use std::ops::{ Deref, DerefMut };
use std::io::Read;
use rocket::{ Request, Data, Outcome };
use rocket::http::{ Status, ContentType };
use rocket::data::{ self, FromDataSimple };
use rocket_contrib::json::JsonValue;
use serde::{ Serialize, Deserialize };
use serde::de::DeserializeOwned;
use ed25519_dalek::{ PublicKey, Signature };

// Implement signature authentication for JSON bodies
#[derive(Debug)]
pub struct SignedRequest<T>(pub T);

impl<T> Deref for SignedRequest<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T {
        &self.0
    }
}
impl<T> DerefMut for SignedRequest<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
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
                    // TODO: Check for unauthorized public keys
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
                        Ok(json) => Outcome::Success(SignedRequest(json)),
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
pub fn initialize(request: SignedRequest<InitializeRequest>) -> JsonValue {
    println!("{}", request.username);
    json!({
        "status": "Pending"
    })
}
