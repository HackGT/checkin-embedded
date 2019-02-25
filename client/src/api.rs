use std::fmt;
use url::Url;
use serde::{ Serialize, Deserialize };
use reqwest::header::{ HeaderName, HeaderValue };
use crate::crypto::Signer;

pub enum Error {
	Network(reqwest::Error),
	Message(&'static str),
}
impl fmt::Debug for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Error::Network(err) => write!(f, "{:?}", err),
			Error::Message(s) => write!(f, "{}", s),
		}
	}
}
impl From<reqwest::Error> for Error {
	fn from(err: reqwest::Error) -> Error {
		Error::Network(err)
	}
}
impl From<&'static str> for Error {
	fn from(err: &'static str) -> Error {
		Error::Message(err)
	}
}

struct SignedRequest {
	body: String,
	header_name: HeaderName,
	header_value: HeaderValue,
}

#[derive(Deserialize, PartialEq, Debug)]
pub enum ManagedStatus {
	Pending,
	Unauthorized,
	AuthorizedHasCredentials,
	AuthorizedNoCredentials,
}
#[derive(Debug)]
pub struct CredentialResponse {
	pub success: bool,
	pub error: Option<String>,
	pub details: Option<String>,
}

pub struct ManagerAPI {
	base_url: Url,
	client: reqwest::Client,
	signer: Signer,
}

impl ManagerAPI {
	#[cfg(debug_assertions)]
	fn base_url() -> &'static str {
		"http://192.168.1.15:3000"
	}
	#[cfg(not(debug_assertions))]
	fn base_url() -> &'static str {
		"https://manager.checkin.hack.gt"
	}

	pub fn new() -> Self {
		let client = reqwest::Client::new();
		let base_url = Url::parse(ManagerAPI::base_url()).expect("Invalid base URL configured");
		Self { base_url, client, signer: Signer::load() }
	}

	fn sign_request<T: Serialize + ?Sized>(&self, request: &T) -> SignedRequest {
		let body = serde_json::to_string_pretty(request).expect("Could not serialize object for signing");

		let signature = self.signer.sign(body.as_bytes());
		let header_value = format!("ed25519 {}/{}", hex::encode(&self.signer.get_public_key()), hex::encode(&signature.to_bytes()[..]));

		SignedRequest {
			body,
			header_name: reqwest::header::AUTHORIZATION,
			header_value: HeaderValue::from_str(&header_value).unwrap(),
		}
	}

	pub fn get_name(&self) -> String {
		crypto_hash::hex_digest(crypto_hash::Algorithm::SHA256, &self.signer.get_public_key())
	}

	pub fn initialize(&self) -> Result<ManagedStatus, Error> {
		#[derive(Serialize)]
		struct Request<'a> {
			username: &'a str,
		}
		#[derive(Deserialize)]
		struct Response {
			status: ManagedStatus,
		}
		let request = Request {
			username: &self.get_name(),
		};
		let signed_request = self.sign_request(&request);

		let response: Response = self.client.post(self.base_url.join("/api/initialize").unwrap())
			.header(signed_request.header_name, signed_request.header_value)
			.header(reqwest::header::CONTENT_TYPE, HeaderValue::from_static("application/json"))
			.body(signed_request.body)
			.send()?
			.json()?;
		Ok(response.status)
	}

	pub fn create_credentials(&self) -> Result<CredentialResponse, Error> {
		#[derive(Serialize)]
		struct Request<'a> {
			username: &'a str,
			password: &'a str,
		}
		#[derive(Deserialize)]
		struct Response {
			success: Option<bool>,
			error: Option<String>,
			details: Option<String>,
		}
		let credentials = self.signer.get_api_credentials();
		let request = Request {
			username: &credentials.username,
			password: &credentials.password,
		};
		let signed_request = self.sign_request(&request);

		let response: Response = self.client.post(self.base_url.join("/api/credentials").unwrap())
			.header(signed_request.header_name, signed_request.header_value)
			.header(reqwest::header::CONTENT_TYPE, HeaderValue::from_static("application/json"))
			.body(signed_request.body)
			.send()?
			.json()?;
		Ok(CredentialResponse {
			success: response.success.unwrap_or(false),
			error: response.error,
			details: response.details,
		})
	}
}
